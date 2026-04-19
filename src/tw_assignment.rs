#![allow(unused_variables)]
#![allow(dead_code)]

use actix_web::{
    App, Error, get, HttpResponse, HttpServer, post, Responder, web,
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    middleware::{from_fn, Logger, Next},
};
use env_logger;
use serde::Deserialize;
use std::sync::Mutex;

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

/* 
1. Buy request comes, sell immediately if there is unused supply otherwise
   record/store incoming buys as "bids" in memory (possibly sorted by price).

2. When sell comes, check stored list of buys and sell starting from the 
   highest price or if no buys store as supply.
   
3. Same price buy requests, first requests buys.
*/

//// ------ Requests
#[derive(Deserialize)]
struct BuyRequest { username: String, volume: u64, price: u64, }

#[derive(Deserialize)]
struct SellRequest { volume: u64, }


//// ----- App State
struct AppState { inner: Mutex<Inner> }

#[derive(Default, Debug)]
struct Inner {
    request_no: u64,
    // .. todo       // allocated 
    supply: u64,     // unallocated 
    bids: Vec<Bid>,
}

#[derive(Debug)]
struct Bid { user: String, volume: u32, price: u32, seq: u64, }


//// ----- Handlers

#[post("/buy")]
async fn buy(state: web::Data<AppState>, req: web::Json<BuyRequest>) -> impl Responder {
    // TODO
    HttpResponse::Ok()
}

#[post("/sell")]
async fn sell(state: web::Data<AppState>, req: web::Json<SellRequest>) -> impl Responder {
    let mut state = state.inner.lock().unwrap();
    state.supply += req.volume;

    format!("state: {state:?}\n ")
}

#[get("/allocation")]
async fn allocation(state: web::Data<AppState>) -> impl Responder {
    // TODO
    HttpResponse::Ok()
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
    })
    .workers(2) // to have a lite program
    .bind(("127.0.0.1", 8080))?
    .run();

    let handle = server.handle();
    server.await?;
 
    println!("Server was shut-down");
    std::io::Result::Ok(())
}