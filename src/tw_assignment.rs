#![allow(dead_code)]
#![allow(unused_imports)]

use actix_web::{
    App, Error, HttpServer, Responder, body::MessageBody, dev::{ServiceRequest, ServiceResponse}, error, get, middleware::{Logger, Next, from_fn}, 
    post, Result, web
};
use serde::{Deserialize, Serialize};
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

#[derive(Deserialize, Serialize)]
struct BuyRequest { user: String, volume: u64, price: u64, }
impl BuyRequest { 
    fn new(user: impl ToString, volume: u64, price: u64) -> Self {
        BuyRequest { user: user.to_string(), volume, price }
    }
}

#[derive(Deserialize, Serialize)]
struct SellRequest { volume: u64, }

#[derive(Clone, Deserialize)]
struct AllocationQuery { username: String }
    // Use Cow ? - No 
    // - Cow avoids allocation when you pass a &str, only allocates when you 
    //   pass a String.
    // - For this program, from HTTP requests — always String
    //   (deserialized from JSON/query params). So Cow brings no benefit here,
    //   String is fine.
    // - And these will make the API more flexible (use in "ctors"):
    // - impl Into<String> — caller can pass &str or String, 
    //   allocation happens inside the function
    // - impl AsRef<str> — caller can pass &str or String, no allocation, 
    //   just borrows

//// ----- App State
#[derive(Debug, Default)]
struct AppState { inner: Mutex<AppStateImpl> }

#[derive(Default, Debug)]
struct AppStateImpl {
    request_no: u64,
    allocations: HashMap<String, u64>,  // allocated 
    supply: u64,                        // unallocated 
    bids: Vec<Bid>,
        // Vec
            // buy : O(n log n) - sort bids vec
            // sell: O(n) - iterate bids vec
        // BTreeMap
            // buy : O(log n) - insert/remove
            // sell: O(k log n) - k removals, k <= n
}

#[derive(Debug)]
struct Bid { user: String, volume: u64, price: u64, seq: u64, }
impl Bid { 
    fn new(user: impl Into<String>, volume: u64, price: u64, seq: u64) -> Self { 
        Self { user: user.into(), volume, price, seq} 
    }
}

//// ----- Handlers

/* 
curl -s -X POST http://localhost:8080/buy -H "Content-Type: application/json" -d "{\"user\":\"u1\",\"volume\":100,\"price\":3}"
curl -s -X POST http://localhost:8080/buy -H "Content-Type: application/json" -d "{\"user\":\"u2\",\"volume\":150,\"price\":2}"
curl -s -X POST http://localhost:8080/buy -H "Content-Type: application/json" -d "{\"user\":\"u3\",\"volume\":50,\"price\":4}"
*/

#[post("/buy")]
async fn buy(
    state: web::Data<AppState>, req: web::Json<BuyRequest>
) -> impl Responder {
    let mut state_ = state.inner.lock().unwrap();
    buy_impl(&mut state_, req.0);

    format!("\nstate: {state_:#?}\n ")

    // format!("{}: {alloc:?}\n", &user) + 
    //     &format!("state: {state:#?}\n ")
}

/// Behavior: register bid; immediately allocate if leftover supply is available.
/// 3. Buy request comes, sell immediately if there is unused supply otherwise
///    store incoming buys as "bids" in memory (possibly sorted by price).
/// Allocation rules
/// + Highest price wins.
/// + FIFO inside a price level (earlier bids at the same price fill first).
/// + Partial fills allowed; unfilled remainder stays open.
/// + Unused supply persists and must auto-match any subsequent bids arriving
///   later.
/// 
/// Big O: n log(n) - due to bids.sort_by
/// 
fn buy_impl(
    state: &mut AppStateImpl,
    buy_req: BuyRequest
) {
    let BuyRequest {user, volume, price} = buy_req;

    // 0. Increment request_no
    state.request_no += 1;
    println!("-- Buy request #{}", state.request_no);

    //// 1. sell immediately if there is unused supply
    if state.supply > 0  {
        // full fill   : state.supply = 60, buy: 50 => supply: 10, bid: 50
        if state.supply >= volume {
            let &alloc = state.allocations.get(&user).unwrap_or(&0);
            state.allocations.insert(user.clone(), alloc + volume);
            state.supply -= volume;
        // partial fill: state.supply = 50, buy: 60 => supply:  0, bid: 10
        } else {  // partial fill: store unfilled as bid
            let state_supply = state.supply;
            let &alloc = state.allocations.get(&user).unwrap_or(&0);
            state.allocations.insert(user.clone(), alloc + state_supply);
            let seq = state.request_no;
            state.bids.push(
                Bid::new(user, volume - state_supply, price, seq)
            );
            // todo: sort? No, bids vec is empty at this point
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
}

/* 
curl -s -X POST localhost:8080/sell -H "Content-Type: application/json" -d "{\"volume\":500}"
*/
#[post("/sell")]
async fn sell(state: web::Data<AppState>, req: web::Json<SellRequest>) -> impl Responder {
    let mut state = state.inner.lock().unwrap();
    sell_impl(&mut state, req.0);

    format!("\nstate: {state:#?}\n ")
}

/// Behavior: add supply and allocate to outstanding bids
/// 2. When sell comes, check stored list of bids and sell starting from the 
///    highest price or if no bids, store as supply.
///
///    Big O: O(n) - due to looping bids
///
fn sell_impl(state: &mut AppStateImpl, sell_req: SellRequest) {
    //// add incoming sell into supply
    state.supply += sell_req.volume;

    //// allocate outstanding bids
    for i in (0..state.bids.len()).rev() {  // O(n)
        if state.supply == 0 { break; }
        let user = state.bids[i].user.clone();
        let bid_volume = state.bids[i].volume;
        // full fill   : state.supply = 60, buy: 50 => supply: 10, bid: 50
        if state.supply >= bid_volume {
            let &alloc = state.allocations.get(&user).unwrap_or(&0);
            state.allocations.insert(user.clone(), alloc + bid_volume);
            state.supply -= bid_volume;
            state.bids.remove(i);           
                // Not O(n) but O(1) - since i always last elem
        // partial fill: state.supply = 50, buy: 60 => supply:  0, bid: 10
        } else {
            let state_supply = state.supply;
            let &alloc = state.allocations.get(&user).unwrap_or(&0);
            state.allocations.insert(user, alloc + state_supply);
            state.bids[i].volume = bid_volume - state_supply;
            state.supply = 0; // we could only partial fill, means supply is 0
            break;
        }
    };

    let total_alloc: u64 = state.allocations.values().sum();
    dbg!(total_alloc);
}

/// Behavior: return the integer total VM-hours allocated to u1 so far.
/// Responses: 200 OK with body like 150, or appropriate 4xx on error 
/// (e.g., missing username).
/*
curl -s localhost:8080/allocation?username=u1
*/
fn allocation_impl(
    state: &AppStateImpl, 
    req: AllocationQuery
) -> Result<u64> {
    state.allocations.get(&req.username)
        .copied()  // Option<&u64> to Option<64>
        // Option to Result
        .ok_or_else(|| error::ErrorBadRequest("missing username\n"))

    // or more readable:
    //
    // if let Some(alloc) = state.allocations.get(&req.username) {
    //     Ok(*alloc)
    // } else {
    //     Err(error::ErrorBadRequest("missing username\n"))
    // }
}

#[get("/allocation")]
async fn allocation(
    state: web::Data<AppState>, 
    req: web::Query<AllocationQuery>
) -> Result<String> {
    let state_ = state.inner.lock().unwrap();
    let res = allocation_impl(&state_, req.0.clone());
    if let Ok(alloc) = res {
        Ok(alloc.to_string())
        // Debug:        
        // Ok( format!("\n{}: {alloc:?}\n", &req.username) + 
        //     &format!("\nstate: {state_:#?}\n ") )
    } else {
        Err(error::ErrorBadRequest("missing username\n"))
    }
}

/// show full app state
async fn index(app_state: web::Data<AppState>) -> String {
    println!("-- thread: {:?}", std::thread::current().id());
    format!("state: {:?}\n", app_state.inner.lock().unwrap())
}

//// ----- Middleware

/// not functional for now
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
    HttpServer::new(move || { // move app_state into the closure
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
    .run()
    .await?;
 
    println!("Server was shut-down");
    std::io::Result::Ok(())
}


mod http_tests {

    use actix_web::{http::StatusCode, test, test::TestRequest};

    use super::*;

    fn test_buy_request(req: BuyRequest) -> actix_http::Request {
        TestRequest::post().uri("/buy").set_json(req).to_request()
    }

    fn test_sell_request(req: SellRequest) -> actix_http::Request {
        TestRequest::post().uri("/sell").set_json(req).to_request()
    }


    // Buy request w/ invalid JSON body returns error 400
    #[actix_web::test]
    async fn test_invalid_json_req_returns_400_error() {
        let state = web::Data::new(AppState::default());
        let app = test::init_service(
            App::new()
                .app_data(state.clone())
                .service(buy)
                .service(sell)
                .service(allocation)
        ).await;

        let req = test::TestRequest::post()
            .uri("/buy")
            .set_payload("invalid json {{{")
            .insert_header(("content-type", "application/json"))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);        
    }

    /*
    Events:
        t1: u1 bids 100 @ 3
        t2: u2 bids 150 @ 2
        t3: u3 bids 50 @ 4
        t4: provider sells 250

    Allocation at t4:
        50 → u3
        100 → u1
        100 → u2 (u2 still open for 50) 
    */    
    #[actix_web::test]
    async fn test_example_in_assignment_doc() {
        let state = web::Data::new(AppState::default());
        let app = test::init_service(
            App::new()
                .app_data(state.clone())
                .service(buy)
                .service(sell)
                .service(allocation)
        ).await;

        //// buy
        let req = test_buy_request(BuyRequest::new("u1", 100, 3));
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let req = test_buy_request(BuyRequest::new("u2", 150, 2));
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let req = test_buy_request(BuyRequest::new("u3", 50, 4));
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        //// sell
        let req = test_sell_request(SellRequest { volume: 250 });
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        //// allocation
        let req = TestRequest::get().uri("/allocation?username=u1").to_request();
        let body: u64 = test::call_and_read_body_json(&app, req).await;
        assert_eq!(body, 100);

        let req = TestRequest::get().uri("/allocation?username=u2").to_request();
        let body: u64 = test::call_and_read_body_json(&app, req).await;
        assert_eq!(body, 100);

        let req = TestRequest::get().uri("/allocation?username=u3").to_request();
        let body: u64 = test::call_and_read_body_json(&app, req).await;
        assert_eq!(body, 50);

        //// bid
        // (u2 still open for 50)
        assert_eq!(state.inner.lock().unwrap().bids[0].volume, 50);
    }

    #[actix_web::test]
    async fn test_basics_buy_sell_and_allocation() {
        let state = web::Data::new(AppState::default());    
        let app = test::init_service(
            App::new()
                .app_data(state.clone())
                .service(buy)
                .service(sell)
                .service(allocation)
        ).await;

        //// buy
        let req = test_buy_request(BuyRequest::new("u1", 100, 2));
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        //// sell
        let req = test_sell_request(SellRequest { volume: 100 });
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        //// allocation
        // good case
        let req = test::TestRequest::get().uri("/allocation?username=u1").to_request();
        let body: u64 = test::call_and_read_body_json(&app, req).await;
        assert_eq!(body, 100);
        // bad cases
        let req = TestRequest::get().uri("/allocation?username=u8").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let req = TestRequest::get().uri("/allocation?username").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }
}

//// -----------  Concurrency Tests
#[cfg(test)]
mod concurrency_tests {
    use std::collections::HashSet;

    use super::*;

    /// send same price buy requests, then
    /// - same price bids with smaller seq numbers should be at the end of 
    ///   bids vector
    /// - check bid vector if seq numbers are unique
    /// - check ordering of same price bids
    #[tokio::test]
    async fn same_price_buy_requests() {
        let state = web::Data::new(AppState::default());

        let handles = (0..5).map(|_| {
            let state = state.clone();
            tokio::spawn(async move {
                let mut state = state.inner.lock().unwrap();
                buy_impl(&mut state, BuyRequest::new("u1", 100, 2));
            })
        });

        for h in handles { h.await.unwrap() }

        let state = state.inner.lock().unwrap();

        // 1. assert no same seq numbers
        let unique: HashSet<_> = state.bids.iter().map(|b| b.seq).collect();
        assert_eq!(state.bids.len(), unique.len());

        // 2. assert ordering of same price bids
        assert_eq!(state.bids.last().unwrap().seq, 1);
        // or
        // check two elem windows
        let seqs = state.bids.iter().map(|b| b.seq).collect::<Vec<u64>>();
        assert!( seqs.windows(2).all(|w| w[0] > w[1]) );
    }

    /// Buys and sells, check allocations + remaining supply = initial supply
    #[tokio::test]
    async fn buy_and_sell() {
        let state = web::Data::new(AppState::default());

        let mut handles = vec![];

        // 50 concurrent buys
        for i in 0..50 {
            let state = state.clone();
            handles.push(tokio::spawn(async move {
                let mut s = state.inner.lock().unwrap();
                buy_impl(&mut s, BuyRequest::new(format!("u{i}"), 10, 1));
            }));
        }

        // 50 concurrent sells
        for _ in 0..50 {
            let state = state.clone();
            handles.push(tokio::spawn(async move {
                let mut s = state.inner.lock().unwrap();
                sell_impl(&mut s, SellRequest { volume: 10 });
            }));
        }

        for h in handles { h.await.unwrap(); }

        let s = state.inner.lock().unwrap();
        let total_allocated: u64 = s.allocations.values().sum();
        let total_supply = 50 * 10; // 500
        assert_eq!(total_allocated + s.supply, total_supply);
    }

    /// claude ai
    #[tokio::test]
    async fn buys_no_oversell_v2() {
        let state = web::Data::new(AppState::default());
    
        let mut handles = vec![];
        for i in 0..100 {
            let state = state.clone();
            handles.push(tokio::spawn(async move {
                let mut s = state.inner.lock().unwrap();
                buy_impl(&mut s, BuyRequest::new(format!("u{i}"), 10, 1));
            }));
        }
        for h in handles { h.await.unwrap(); }

        sell_impl(&mut state.inner.lock().unwrap(), SellRequest { volume: 500 });

        let state = state.inner.lock().unwrap();
        let total_allocated: u64 = state.allocations.values().sum();
        assert_eq!(total_allocated + state.supply, 500);
    }

    #[tokio::test]
    async fn buys_no_oversell() {
        let total_supply = 500;
        let state = web::Data::new(
            AppState { inner: Mutex::new( 
                AppStateImpl { supply: total_supply, ..Default::default() } 
            )}
        );

        let handles = (0..100).map(|_| {
            let state = state.clone();
            tokio::spawn(async move {
                let mut state = state.inner.lock().unwrap();
                buy_impl(&mut state, BuyRequest::new("u1", 200, 2));
            })
        });

        for h in handles { h.await.unwrap(); }
    
        let state = state.inner.lock().unwrap();
        let total_allocated: u64 = state.allocations.values().sum();
        let leftover_supply = state.supply;
        assert_eq!(total_supply, total_allocated + leftover_supply);
    }

}


//// -----------  Property Tests


#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {

    /// supply added via /sell = total allocated + remaining supply 
    #[test]
    fn supply_conservation(
        // e.g. vec![1, 100, 5]
        supplies in prop::collection::vec(1u64..10_000, 1..10),
        // bid: (volume, price), e.g. vec![(100,1), (50,4)]
        bids in prop::collection::vec( (1u64..1_000, 1u64..100), 1..10 ),
    ) {
        let mut state = AppStateImpl::default();
        let total_supply: u64 = supplies.iter().sum();

        for (volume, price) in bids {
            buy_impl(&mut state, BuyRequest::new("u1", volume, price));
        }

        for supply in supplies {
            sell_impl(&mut state, SellRequest { volume: supply });
        }

        let total_allocated = state.allocations.values().sum::<u64>();
        prop_assert_eq!(
            total_supply,
            total_allocated + state.supply
        );
    }

    /// Early arrived request will be filled first for same price requests
    #[test]
    fn fifo_within_same_price(
        supply in 1u64..10_000,
        price in 1u64..50,
        volume in 1u64..1_000,
    ) {
        let mut state = AppStateImpl::default();
        buy_impl(&mut state, BuyRequest::new("u1", volume, price));
        buy_impl(&mut state, BuyRequest::new("u2", volume, price));
        sell_impl(&mut state, SellRequest { volume: supply });
        
        let u1_alloc = state.allocations.get("u1").copied().unwrap_or(0);
        let u2_alloc = state.allocations.get("u2").copied().unwrap_or(0);
    
        // u1 should fill before u2
        prop_assert!(u1_alloc >= u2_alloc);
    }

    /// Higher price requests will be filled first
    #[test]
    fn higher_price_always_fills_first(
        supply in 1u64..10_000,
        lo_price in 1u64..50,
        hi_price in 51u64..100,
        volume in 1u64..1_000,
    ) {
        let mut state = AppStateImpl::default();
        buy_impl(&mut state, BuyRequest::new("lo", volume, lo_price));
        buy_impl(&mut state, BuyRequest::new("hi", volume, hi_price));
        sell_impl(&mut state, SellRequest { volume: supply });
        
        let lo_alloc = state.allocations.get("lo").copied().unwrap_or(0);
        let hi_alloc = state.allocations.get("hi").copied().unwrap_or(0);
        
        // hi should fill before lo
        prop_assert!(hi_alloc >= lo_alloc);
    }

    #[test]
    fn allocated_never_exceeds_supply(
        supply in 0u64..10_000,
        volume in 0u64..10_000,
        price in 1u64..100
    ) {
        let mut state = AppStateImpl { supply, ..Default::default() };
        buy_impl(&mut state, BuyRequest::new("u1", volume, price));
        let allocated = state.allocations.get("u1").copied().unwrap_or(0);
        prop_assert!(allocated <= supply);
    }

    #[test]
    fn partial_fill_remainder_stays_open(
        supply in 1u64..10_000, 
        volume in 2u64..10_000, 
        price in 1u64..100
    ) {
        prop_assume!(supply < volume); // force partial fill
        let mut state = AppStateImpl { supply, ..Default::default() };
        buy_impl(&mut state, BuyRequest::new("u1", volume, price));
        prop_assert_eq!(state.supply, 0);
        prop_assert!(!state.bids.is_empty()); // remainder stays open
    }

    #[test]
    fn allocation_monotonically_increases(
        supplies in prop::collection::vec(1u64..10_000, 1..10),
            // 1–10 random elements, each between 1 and 10_000. 
            // e.g. [500, 3200, 77] or [9999] or [1, 200, 50, 8000]
        bids in prop::collection::vec((1u64..10_000, 1u64..100), 1..10),
            // e.g. [(100, 3), (5000, 7), (200, 1)], each tuple (volume, price)
    ) {
        let mut state = AppStateImpl::default();
        let mut prev_alloc = 0u64;

        // each time: 
        // - buy() : u1 bids
        // - sell(): supply becomes available, u1 gets allocation  
        // e.g.
        // bids    : [(100, 3), (5000, 7), (200, 1)], each tuple (volume, price)
        // supplies: [9999] 
        //
        for (volume, price) in bids {
            buy_impl(&mut state, BuyRequest::new("u1", volume, price));
            for supply in &supplies {
                sell_impl(&mut state, SellRequest { volume: *supply });
            }
            let alloc = state.allocations.get("u1").copied().unwrap_or(0);
            prop_assert!(alloc >= prev_alloc); // never decreases
            prev_alloc = alloc;
        }
    }

    } // end of macro proptest!
}


//// -----------  Unit Tests

#[cfg(test)]
mod unit_tests {
    use actix_web::http::StatusCode;

    use super::*;


    // Example
    //     Events:
    //
    //     t1: u1 bids 100 @ 3
    //     t2: u2 bids 150 @ 2
    //     t3: u3 bids 50 @ 4
    //     t4: provider sells 250
    //     Allocation at t4:

    //     50 → u3
    //     100 → u1
    //     100 → u2 (u2 still open for 50)
    #[test]
    fn unused_supply_auto_sold() {
        let mut state = AppStateImpl::default();
        buy_impl(&mut state, BuyRequest::new("u1", 100, 3));
        buy_impl(&mut state, BuyRequest::new("u2", 150, 2));
        buy_impl(&mut state, BuyRequest::new("u3", 50, 4));
        sell_impl(&mut state, SellRequest { volume: 250 });
        assert_eq!(state.allocations.get("u3").unwrap(), &50);
        assert_eq!(state.allocations.get("u1").unwrap(), &100);
        assert_eq!(state.allocations.get("u2").unwrap(), &100);
        let u2 = state.bids.iter().find(|bid| bid.user == "u2").unwrap();
        assert_eq!(u2.volume, 50);
    }

    #[test]
    fn allocation() {
        let state = AppStateImpl { 
            allocations: HashMap::from( [("u1".to_string(), 100)] ), 
            ..Default::default()
        };

        // - good case
        let result = allocation_impl(
            &state, AllocationQuery { username: "u1".to_string() }
        ).unwrap();
        assert_eq!(result, 100);

        // - error case
        let result = allocation_impl(
            &state, AllocationQuery { username: "u2".to_string() }
        );
        let status = result.as_ref().unwrap_err().error_response().status();
        assert_eq!(status, StatusCode::BAD_REQUEST);
        let err_string = result.unwrap_err().to_string();
        assert_eq!(err_string, "missing username\n");
    }

    #[test]
    fn sell() {
        // 1. add incoming sell into supply
        let mut state = AppStateImpl::default();
        assert_eq!(state.supply, 0);
        sell_impl(&mut state, SellRequest { volume: 400 });
        assert_eq!(state.supply, 400);
    
        // 2. allocate outstanding bids
        // case: full fill - state.supply = 60, buy: 50 => supply: 10, bid: 50
        let mut state = AppStateImpl { 
            bids: vec![ Bid::new("u1", 200, 2, 1) ],
            ..Default::default()
        };
        sell_impl(&mut state, SellRequest { volume: 300 });
        assert_eq!(state.allocations.get("u1").unwrap(), &200);
        assert_eq!(state.supply, 100);
        assert!(state.bids.is_empty());
        // case: partial fill: state.supply = 50, buy: 60 => supply:  0, bid: 10
        let mut state = AppStateImpl { 
            bids: vec![ Bid::new("u1", 100, 2, 1) ],
            ..Default::default()
        };
        sell_impl(&mut state, SellRequest { volume: 50 });
        assert_eq!(state.allocations.get("u1").unwrap(), &50);
        assert_eq!(state.supply, 0);
        assert_eq!(state.bids[0].user, "u1");
        assert_eq!(state.bids[0].volume, 50);
        assert_eq!(state.bids[0].price, 2);
    }

    #[test]
    fn buy() {
        //// 1. sell immediately if there is unused supply
        // full fill
        let mut state = AppStateImpl { supply: 200, ..Default::default() };
        buy_impl(&mut state, BuyRequest::new("u1", 200, 2));
        assert_eq!(state.request_no, 1);
        assert_eq!(state.allocations.get("u1").unwrap(), &200);
        assert_eq!(state.supply, 0);

        // partial fill
        let mut state = AppStateImpl { supply: 50, ..Default::default() };
        let buy_req = BuyRequest::new("u1", 100, 2);
        buy_impl(&mut state, buy_req);
        assert_eq!(state.request_no, 1);
        assert_eq!(state.supply, 0);
        assert_eq!(state.allocations.get("u1").unwrap(), &50);
        assert_eq!(state.bids.len(), 1);
        assert_eq!(state.bids[0].volume, 50);
        assert_eq!(state.bids[0].price, 2);
        assert_eq!(state.bids[0].seq, 1);

        //// 2. otherwise, store req into bids
        let mut state = AppStateImpl::default();

        // case: basic first bid 
        buy_impl(&mut state, BuyRequest::new("u1", 100, 2));
        assert_eq!(state.request_no, 1);
        assert_eq!(state.bids.len(), 1);
        assert_eq!(state.bids[0].volume, 100);
        assert_eq!(state.bids[0].price, 2);
        assert_eq!(state.bids[0].seq, 1);
        // case: earlier bids at the same price fill first
        buy_impl(&mut state, BuyRequest::new("u2", 100, 2));
        assert_eq!(state.request_no, 2);
        assert_eq!(state.bids.len(), 2);
        assert_eq!(state.bids[0].volume, 100);
        assert_eq!(state.bids[0].price, 2);
        assert_eq!(state.bids[0].seq, 2);
        assert_eq!(state.bids[1].user, "u1");  // u1 bid first
        assert_eq!(state.bids[1].seq, 1);
        // case: highest price always wins
        buy_impl(&mut state, BuyRequest::new("u3", 100, 3));
        assert_eq!(state.request_no, 3);
        assert_eq!(state.bids.len(), 3);
        assert_eq!(state.bids.last().unwrap().user, "u3");
    }

    /// Higher price requests will be filled first
    #[test]
    fn higher_price_always_fills_first() {
        let mut state = AppStateImpl::default();
        buy_impl(&mut state, BuyRequest::new("u1", 200, 2));
        buy_impl(&mut state, BuyRequest::new("u2", 200, 4));
        buy_impl(&mut state, BuyRequest::new("u3", 200, 10));
        sell_impl(&mut state, SellRequest { volume: 300 });
    
    }

    /// Same user buys twice
    /// - allocation should accumulate, not overwrite
    /// - bids should be separate and unique
    #[test]
    fn buy_same_user_buys_twice() {
        //// 1. sell immediately if there is unused supply
        // full fill
        let mut state = AppStateImpl { supply: 400, ..Default::default() };
        let buy_req = BuyRequest::new("u1", 200, 2);
        buy_impl(&mut state, buy_req);
        let buy_req = BuyRequest::new("u1", 200, 2);
        buy_impl(&mut state, buy_req);
        assert_eq!(state.allocations.get("u1").unwrap(), &400);

        // partial fill
        let mut state = AppStateImpl { supply: 300, ..Default::default() };
        let buy_req = BuyRequest::new("u1", 200, 2);
        buy_impl(&mut state, buy_req);
        let buy_req = BuyRequest::new("u1", 200, 2);
        buy_impl(&mut state, buy_req);
        assert_eq!(state.allocations.get("u1").unwrap(), &300);

        //// 2. otherwise, store req into bids
        let mut state = AppStateImpl::default();

        buy_impl(&mut state, BuyRequest::new("u1", 100, 2));
        buy_impl(&mut state, BuyRequest::new("u1", 100, 2));
        assert_eq!(state.bids.len(), 2);
        assert_eq!(state.bids[0].user, "u1");
        assert_eq!(state.bids[1].user, "u1");
    }
}

