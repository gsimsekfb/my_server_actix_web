#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(unused_imports)]

use actix_web::{
    App, Error, HttpServer, Responder, body::MessageBody, dev::{ServiceRequest, ServiceResponse}, error, get, middleware::{Logger, Next, from_fn}, 
    post, Result, web
};
use serde::Deserialize;
use std::{collections::HashMap, sync::{Mutex, MutexGuard}};

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
impl BuyRequest { 
    fn new(user: impl ToString, volume: u64, price: u64) -> Self {
        BuyRequest { user: user.to_string(), volume, price }
    }
}

#[derive(Deserialize)]
struct SellRequest { volume: u64, }

#[derive(Clone, Deserialize)]
struct AllocationQuery { username: String }
    // todo: use Cow ?


//// ----- App State
#[derive(Default)]
struct AppState { inner: Mutex<AppStateImpl> }

#[derive(Default, Debug)]
struct AppStateImpl {
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
/// Allocation rules
/// + Highest price wins.
/// + FIFO inside a price level (earlier bids at the same price fill first).
/// + Partial fills allowed; unfilled remainder stays open.
/// + Unused supply persists and must auto-match any subsequent bids arriving later.

#[post("/buy")]
async fn buy(
    state: web::Data<AppState>, req: web::Json<BuyRequest>
) -> impl Responder {
    let state = buy_impl(state, req.0);
    let state = state.inner.lock().unwrap();
    format!("\nstate: {state:#?}\n ")

    // format!("{}: {alloc:?}\n", &user) + 
    //     &format!("state: {state:#?}\n ")
}

fn buy_impl(
    state_: web::Data<AppState>, buy_req: BuyRequest
) -> web::Data<AppState> {
    let mut state = state_.inner.lock().unwrap();
    let BuyRequest {user, volume, price} = buy_req;

    // 0. Increment request_no
    state.request_no += 1;
    println!("-- Buy request #{}", state.request_no);

    //// 1. sell immediately if there is unused supply
    let buy_value = price * volume;
    if state.supply > 0  {
        // full fill   : state.supply = 60, buy: 50 => supply: 10, bid: 50
        if state.supply >= buy_value {
            let &alloc = state.allocations.get(&user).unwrap_or(&0);
            state.allocations.insert(user.clone(), alloc + buy_value);
            state.supply -= buy_value;
        // partial fill: state.supply = 50, buy: 60 => supply:  0, bid: 10
        } else {  // partial fill: store unfilled as bid
            let state_supply = state.supply;
            let &alloc = state.allocations.get(&user).unwrap_or(&0);
            state.allocations.insert(user.clone(), alloc + state_supply);
            let seq = state.request_no;
            state.bids.push(
                Bid::new(user, (buy_value - state_supply) / price, price, seq)
            );
            state.supply = 0;
        }
    }
    //// 2. otherwise, store req into bids
    ////    - highest price bid at the end of bids vector
    ////    - same price bids, early bid stored at the end of bids vector 
    else {
        let seq = state.request_no;
        state.bids.push(Bid::new(user, volume, price, seq));
        state.bids.sort_by(|a, b| a.price.cmp(&b.price).then(b.seq.cmp(&a.seq)));
    }
    drop(state);

    state_
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
    let mut state = state.inner.lock().unwrap();
    for i in (0..state.bids.len()).rev() {
        let user = state.bids[i].user.clone();
        let bid_value = state.bids[i].price * state.bids[i].volume;
        let bid_price = state.bids[i].price;
        // full fill   : state.supply = 60, buy: 50 => supply: 10, bid: 50
        if state.supply >= bid_value {
            let &alloc = state.allocations.get(&user).unwrap_or(&0);
            state.allocations.insert(user.clone(), alloc + bid_value);
            state.supply -= bid_value;
            state.bids.remove(i);
        // partial fill: state.supply = 50, buy: 60 => supply:  0, bid: 10
        } else {
            let state_supply = state.supply;
            let &alloc = state.allocations.get(&user).unwrap_or(&0);
            state.allocations.insert(user, alloc + state_supply);
            state.bids[i].volume = (bid_value - state_supply) / bid_price;
            state.supply = 0;
        }
    };

    let total_alloc: u64 = state.allocations.values().sum();
    dbg!(total_alloc);

    format!("\nstate: {state:#?}\n ")
}

/// Behavior: return the integer total VM-hours allocated to u1 so far.
/// Responses: 200 OK with body like 150, or appropriate 4xx on error 
/// (e.g., missing username).
/*
curl -s localhost:8080/allocation?username=u1
*/
fn allocation_impl(
    state: MutexGuard<'_, AppStateImpl>, 
    req: AllocationQuery
) -> Result<u64> {
    // todo: refactor
    if let Some(alloc) = state.allocations.get(&req.username) {
        Ok(*alloc)
    } else {
        Err(error::ErrorBadRequest("missing username\n"))
    }
}

#[get("/allocation")]
async fn allocation(
    state: web::Data<AppState>, 
    req: web::Query<AllocationQuery>
) -> Result<String> {
    let state_ = state.inner.lock().unwrap();
    let res = allocation_impl(state_, req.0.clone());

    let state_ = state.inner.lock().unwrap();
    if let Ok(alloc) = res {
        Ok( format!("\n{}: {alloc:?}\n", &req.username) + 
            &format!("\nstate: {state_:#?}\n ") )
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
    // ...

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
        AppState { inner: Mutex::new( AppStateImpl::default() ) }
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

//// -----------  Unit Tests

#[cfg(test)]
mod tests {
    use actix_web::http::StatusCode;

    use super::*;
    
    #[test]
    fn allocation() {
        let state = web::Data::new( AppState {
            inner: Mutex::new(AppStateImpl { 
                allocations: HashMap::from( [("u1".to_string(), 100)] ), 
                ..Default::default()
            })
        });

        // - good case
        let state_ = state.inner.lock().unwrap();
        let result = allocation_impl(
            state_, AllocationQuery { username: "u1".to_string() }
        ).unwrap();
        assert_eq!(result, 100);

        // - error case
        let state_ = state.inner.lock().unwrap();
        let result = allocation_impl(
            state_, AllocationQuery { username: "u2".to_string() }
        );
        let status = result.as_ref().unwrap_err().error_response().status();
        assert_eq!(status, StatusCode::BAD_REQUEST);
        let err_string = result.unwrap_err().to_string();
        assert_eq!(err_string, "missing username\n");
    }

    #[test]
    fn sell() {
        // todo
    }

    #[test]
    fn buy() {
        //// 1. sell immediately if there is unused supply
        // full fill
        let state = web::Data::new( AppState {
            inner: Mutex::new(AppStateImpl { supply: 200, ..Default::default() })
        });
        let buy_req = BuyRequest::new("u1", 100, 2);
        let state = buy_impl(state, buy_req);
        let state = state.inner.lock().unwrap();
        assert_eq!(state.request_no, 1);
        assert_eq!(state.allocations.get("u1").unwrap(), &200);
        assert_eq!(state.supply, 0);

        // partial fill
        let state = web::Data::new( AppState {
            inner: Mutex::new(AppStateImpl { supply: 50, ..Default::default() })
        });
        let buy_req = BuyRequest::new("u1", 100, 2);
        let state = buy_impl(state, buy_req);
        let state = state.inner.lock().unwrap();
        assert_eq!(state.request_no, 1);
        assert_eq!(state.supply, 0);
        assert_eq!(state.allocations.get("u1").unwrap(), &50);
        assert_eq!(state.bids.len(), 1);
        assert_eq!(state.bids[0].volume, 75);
        assert_eq!(state.bids[0].price, 2);
        assert_eq!(state.bids[0].seq, 1);

        //// 2. otherwise, store req into bids
        let state = web::Data::new( AppState::default() );

        // case: basic first bid 
        let buy_req = BuyRequest::new("u1", 100, 2);
        let state = buy_impl(state, buy_req);
        {   
            let state_ = state.inner.lock().unwrap();
            assert_eq!(state_.request_no, 1);
            assert_eq!(state_.bids.len(), 1);
            assert_eq!(state_.bids[0].volume, 100);
            assert_eq!(state_.bids[0].price, 2);
            assert_eq!(state_.bids[0].seq, 1);
        }    
        // case: earlier bids at the same price fill first
        let buy_req = BuyRequest::new("u2", 100, 2);
        let state = buy_impl(state, buy_req);
        {
            let state_ = state.inner.lock().unwrap();
            assert_eq!(state_.request_no, 2);
            assert_eq!(state_.bids.len(), 2);
            assert_eq!(state_.bids[0].volume, 100);
            assert_eq!(state_.bids[0].price, 2);
            assert_eq!(state_.bids[0].seq, 2);
            assert_eq!(state_.bids[1].user, "u1");  // u1 bid first
            assert_eq!(state_.bids[1].seq, 1);
        }
        // case: highest price always wins
        let buy_req = BuyRequest::new("u3", 100, 3);
        let state = buy_impl(state, buy_req);
        {
            let state_ = state.inner.lock().unwrap();
            assert_eq!(state_.request_no, 3);
            assert_eq!(state_.bids.len(), 3);
            assert_eq!(state_.bids.last().unwrap().user, "u3");
        }
    }
}

