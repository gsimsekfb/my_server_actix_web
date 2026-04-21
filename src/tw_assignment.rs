#![allow(unused_variables)]
#![allow(dead_code)]

use actix_web::{
    App, Error, HttpServer, Responder, body::MessageBody, dev::{ServiceRequest, ServiceResponse}, error, get, middleware::{Logger, Next, from_fn}, 
    post, Result, web
};
use serde::Deserialize;
use std::{collections::HashMap, sync::Mutex};

/* Cheatsheet
.
watchexec -e rs -r -- cargo run --bin main
watchexec -e rs -r -- cargo run --bin main --release
set RUST_LOG=actix_web=debug && watchexec -e rs -r -- cargo run --bin main --release

curl -s -X POST localhost:8080
curl -is -X POST localhost:8080
curl -s -X POST localhost:8080/sell -H "Content-Type: application/json" -d "{\"volume\":250}"
.
*/

//// ------ Requests
#[derive(Deserialize)]
struct BuyRequest { user: String, volume: u64, price: u64, }

#[derive(Deserialize)]
struct SellRequest { volume: u64, }

#[derive(Deserialize)]
struct AllocationQuery { username: String }


//// ----- App State
struct AppState { inner: Mutex<Inner> }

#[derive(Default, Debug)]
struct Inner {
    request_no: u64,
    allocations: HashMap<String, u64>,  // allocated 
    supply: u64,                        // unallocated 
    bids: Vec<Bid>,
}

#[derive(Debug)]
struct Bid { user: String, volume: u64, price: u64, seq: u64, }
impl Bid { 
    fn new(user: String, volume: u64, price: u64, seq: u64) -> Self { 
        Self { user, volume, price, seq} 
    }
}

//// ----- Handlers

/* 
curl -s -X POST http://localhost:8080/buy -H "Content-Type: application/json" -d "{\"user\":\"u1\",\"volume\":100,\"price\":3}"
curl -s -X POST http://localhost:8080/buy -H "Content-Type: application/json" -d "{\"user\":\"u2\",\"volume\":150,\"price\":2}"
curl -s -X POST http://localhost:8080/buy -H "Content-Type: application/json" -d "{\"user\":\"u3\",\"volume\":50,\"price\":4}"
*/
/// Behavior: register bid; immediately allocate if leftover supply is available.
/// 3. Buy request comes, sell immediately if there is unused supply otherwise
///    store incoming buys as "bids" in memory (possibly sorted by price).
///    - Same price buy requests, first requests buys.
#[post("/buy")]
async fn buy(state: web::Data<AppState>, req: web::Json<BuyRequest>) -> impl Responder {
    let mut state = state.inner.lock().unwrap();
    let BuyRequest {user, volume, price} = req.into_inner();

    // sell immediately if there is unused supply
    let buy_value = price * volume;
    if state.supply > buy_value  {
        state.allocations.insert(user.clone(), buy_value);
        state.supply -= buy_value;
    }
    // otherwise, store req into bids
    else {
        let seq = state.request_no;
        state.bids.push(Bid::new(user, volume, price, seq));
        state.bids.sort_by(|a, b| a.price.cmp(&b.price));
    }

    format!("\nstate: {state:#?}\n ")

    // format!("{}: {alloc:?}\n", &user) + 
    //     &format!("state: {state:#?}\n ")
}

/// Behavior: add supply and allocate to outstanding bids
/// 2. When sell comes, check stored list of bids and sell starting from the 
//     highest price or if no bids, store as supply.
/* 
curl -s -X POST localhost:8080/sell -H "Content-Type: application/json" -d "{\"volume\":500}"
*/
#[post("/sell")]
async fn sell(state: web::Data<AppState>, req: web::Json<SellRequest>) -> impl Responder {
    //// add incoming sell into supply
    {
        let mut state = state.inner.lock().unwrap();
        state.supply += req.volume;
    }
    
    //// allocate outstanding bids
    // find bids_to_process
    let state_ = state.inner.lock().unwrap();
    let bids = &state_.bids;
    let mut supply = state_.supply;
    let bids_to_process: Vec<usize> = state_.bids
        .iter()
        .enumerate()
        .rev()
        .inspect(|b| println!("{b:?}"))
        .filter(|(i, bid)| { 
            let buy_val = bid.price * bid.volume;
            if buy_val <= supply { supply -= buy_val; return true }
            false
        })
        .map(|(i, _)| i)
        .collect();
    drop(state_);

    dbg!(&bids_to_process);
    dbg!(supply);

    // allocated bids_to_process
    let mut state = state.inner.lock().unwrap();
    bids_to_process.iter().for_each(|&i| {
        let user = state.bids[i].user.clone();
        let bid_value = state.bids[i].price * state.bids[i].volume;
        state.allocations.insert(user, bid_value);
        state.supply -= bid_value;
        state.bids.remove(i);
    });

    format!("\nstate: {state:#?}\n ")
}

/// Behavior: return the integer total VM-hours allocated to u1 so far.
/// Responses: 200 OK with body like 150, or appropriate 4xx on error (e.g., missing username).
/*
curl -s localhost:8080/allocation?username=u1
*/
#[get("/allocation")]
async fn allocation(
    state: web::Data<AppState>, 
    req: web::Query<AllocationQuery>
) -> Result<String> {
    let state = state.inner.lock().unwrap();
    if let Some(alloc) = state.allocations.get(&req.username) {
        Ok( format!("\n{}: {alloc:?}\n", &req.username) + 
            &format!("\nstate: {state:#?}\n ") )
    } else {
        Err(error::ErrorBadRequest("missing username\n"))
    }
}

async fn index(app_state: web::Data<AppState>) -> String {
    println!("-- thread: {:?}", std::thread::current().id());
    format!("state: {:?}\n", app_state.inner.lock().unwrap())
}

//// ----- Middleware

async fn my_middleware(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    // step-1. pre-processing
    {
        let state = req.app_data::<web::Data<AppState>>().unwrap();
        let mut state = state.inner.lock().unwrap();
        state.request_no += 1;
        println!("-- Request #{}", state.request_no);
    }
    // step-2: call handler
    next.call(req).await

    // step-3. post-processing
    // ...
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("-- Server starting on localhost:8080 ...");
    println!("-- main's thread: {:?}", std::thread::current().id());

    env_logger::init();

    // web::Data<T> is struct Data<T>(Arc<T>)
    let app_state = web::Data::new(
        AppState { inner: Mutex::new( Inner::default() ) }
    );

    // closure will be run per worker thread (at startup), default workers: 8
    let server = HttpServer::new(move || { // move app_state into the closure
        App::new()
            .wrap(Logger::default())
            // clone for each worker thread
            .app_data(app_state.clone()) // register the created data
            .route("/", web::get().to(index))
            .wrap(from_fn(my_middleware))
            .service(sell)
            .service(buy)
            .service(allocation)
    })
    .workers(2) // to have a lite program
    .bind(("127.0.0.1", 8080))?
    .run();

    let handle = server.handle();
    server.await?;
 
    println!("Server was shut-down");
    std::io::Result::Ok(())
}