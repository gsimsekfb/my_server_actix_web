use actix_web::{
    App, Error, HttpRequest, HttpResponse, HttpServer, Responder, Result, body::MessageBody, dev::{ServiceRequest, ServiceResponse}, error, get, middleware::{Logger, Next, from_fn}, post, web
};
use dashmap::DashMap;
use metrics::{counter, gauge, histogram};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use std::{
    cmp::Reverse, collections::BTreeMap, sync::{
        Arc, Mutex, MutexGuard, RwLock, RwLockWriteGuard, atomic::{AtomicU64, Ordering}
    },
    time::Instant
};

use super::tw_auth::decode_jwt;
use super::tw_error::ErrorResponse;
use super::tw_err_circuit_breaker::{PriceFeed, PriceFeedError};


/// Topics
/// 1. Lock order - deadlock prevention enforced with fns and with AI pre-commit hook
///    - see ordered_locks_*() fns
///    - see tw_ai_pre_commit_hook.txt
/// 2. Full separation between HTTP layer and business logic 
///    - See buy and buy_impl fns
/// 3. Granular per AppState field locks vs one Mutex for all AppState designs
///    - See commit: "tw: Using lock/lockfree per AppState field instead of one 
///      Mutex for all AppState"
/// 4. Using Vec (manual sort) vs BTreeMap (auto sort) for sorted bids
///
/// See tw__readme.md for more


/// Perf. optimization version:
/// v4
//   Requests/sec: 51780
//   99% in 0.0005 secs
//   Change: AppState.supply is now AtomicU64 instead of Mutex<u64> 


// --------------------------------------------------------------------------


//// ------ Requests

#[derive(Debug, Deserialize, Serialize)]
pub struct BuyRequest { user: String, volume: u64, price: u64, }

impl BuyRequest {
    pub fn new(user: impl ToString, volume: u64, price: u64) -> Self {
        BuyRequest { user: user.to_string(), volume, price }
    }
}

#[derive(Deserialize, Serialize)]
pub struct SellRequest { pub volume: u64, }

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

#[derive(Default, Debug)]
pub struct AppState {
    pub buy_seq_no: AtomicU64, // buy sequence number
    pub supply: AtomicU64,                 // unallocated 
        // this could be Atomic, using Mutex to show lock order / deadlock topic
    pub allocations: DashMap<String, u64>,  // allocated 
        // DashMap: lock-free concurrent HashMap which uses Mutex sharding
    // Highest price top element. For same price bids, smaller seq on top
    // (Reverse(price), seq)
    bids: RwLock<BTreeMap<PriceSeqPair, Bid>>, // RwLock for educ. purposes
        // Mutex when frequently modified/written
        // RwLock when frequently read (reads 10 times more than writes)
        //  Vec<Bid>
            // buy : O(n log n) - sort bids vec
            // sell: O(n) - iterate bids vec
        // BTreeMap<PriceSeqPair, Bid>
            // buy : O(log n) - insert
            // sell: O(n) - retain will visit every elem
    price_feed: Arc<PriceFeed>
}

type PriceSeqPair = (Reverse<u64>, u64);

#[cfg(test)]
fn price_seq_pair(price: u64, seq: u64) -> (Reverse<u64>, u64) {
    (Reverse(price), seq)
}

#[allow(unused)]
#[derive(Debug)]
pub struct Bid { user: String, volume: u64, price: u64, seq: u64, }
impl Bid { 
    fn new(user: impl Into<String>, volume: u64, price: u64, seq: u64) -> Self { 
        Self { user: user.into(), volume, price, seq} 
    }
}


//// ----- Ordered Locks
////
//// To avoid deadlock, the lock order must be the same in these fns 
//// 
//// Also see: pre-commit hook ai check lock order for deadlock:
//// src\tw_ai_pre_commit_hook.txt

pub fn ordered_locks_buy(state: &AppState) -> 
    RwLockWriteGuard<'_, BTreeMap<PriceSeqPair, Bid>>
{
    // let supply = state.supply.lock().unwrap();
    state.bids.write().unwrap()
}

pub fn ordered_locks_sell(state: &AppState) -> 
    RwLockWriteGuard<'_, BTreeMap<PriceSeqPair, Bid>>
{
    state.bids.write().unwrap()
}


//// ----- Handlers

/* 
curl -s -X POST http://localhost:8080/buy -H "Content-Type: application/json" -d "{\"user\":\"u1\",\"volume\":100,\"price\":3}"
curl -s -X POST http://localhost:8080/buy -H "Content-Type: application/json" -d "{\"user\":\"u2\",\"volume\":150,\"price\":2}"
curl -s -X POST http://localhost:8080/buy -H "Content-Type: application/json" -d "{\"user\":\"u3\",\"volume\":50,\"price\":4}"
*/
#[post("/buy")]
#[instrument(skip(state))]
async fn buy(
    state: web::Data<AppState>,
    req_http: HttpRequest,
    req: web::Json<BuyRequest>
) -> impl Responder {

    //// Pre API versioning workaround using JWT feature flag.
    //// Assuming "new_allocation" JWT token flag, calls a new buy_impl.
    //// See tw_api_versioning.md/.rs for more.
    let token = req_http
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));    
    
    // let _ = dbg!(decode_jwt(token.unwrap()));
    
    let has_feature_new_alloc = token
        .and_then(|t| decode_jwt(t).ok())
        .map(|claims| claims.features.contains(&"new_allocation".to_string()))
        .unwrap_or(false);

    if has_feature_new_alloc {
        // buy_impl_new_alloc() // todo
    }

    //// buy starts here
    let mut bids = ordered_locks_buy(&state);
    // instead of:
        // let mut supply = state.supply.lock().unwrap();
        // let mut bids = state.bids.write().unwrap();

    buy_impl(
        &state.buy_seq_no,
        &state.supply,
        &state.allocations,
        &mut bids,
        req.0
    );

    HttpResponse::Ok().finish()
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
/// Big O: log N - btreemap insert
/// 
#[instrument(skip(buy_seq_no, supply, allocations, bids))]
pub fn buy_impl(
    buy_seq_no: &AtomicU64,
    supply: &AtomicU64,
    allocations: &DashMap<String, u64>,
    bids: &mut BTreeMap<PriceSeqPair, Bid>, 
    buy_req: BuyRequest
) {
    // metrics
    let start = Instant::now();

    let BuyRequest {user, volume, price} = buy_req;

    // 0. Increment request_no
    buy_seq_no.fetch_add(1, Ordering::Relaxed);
    // println!("-- Buy sequence number: #{buy_seq_no:?}");

    let mut current = supply.load(Ordering::Acquire);
    loop { // CAS loop

        //// 1. No supply, early return, store req into bids
        ////    - highest price bid at the end of bids vector
        ////    - same price bids, early bid stored at the end of bids vector 
        if current == 0 {
            let seq = buy_seq_no.load(Ordering::Relaxed);
            bids.insert(
                (Reverse(price), seq), 
                Bid::new(user.clone(), volume, price, seq)
            );

            // metrics — early-exit path
            // todo: use RAII e.g. BuyMetricsGuard fn drop
            histogram!("buy_impl_duration_seconds")
                .record(start.elapsed().as_secs_f64());
            counter!("http_requests_total", "endpoint" => "buy").increment(1);
            gauge!("open_bids_count").set(bids.len() as f64);
            gauge!("supply_current").set(supply.load(Ordering::Relaxed) as f64);

            return;
        }

        //// 2. There is supply, sell immediately if there is unused supply
        let new_val = current.saturating_sub(volume);
            // if current >= volume { current - volume } // full fill 
            // else { 0 };                               // partial fill
        match supply.compare_exchange(
            current, new_val, Ordering::AcqRel, Ordering::Acquire
        ) {
            Ok(supply_) => { // we won, value updated, 
                            // supply_ is NOT new_value yet
                if supply_ >= volume { // full fill 
                    let current_alloc = 
                        *allocations.get(&user).as_deref().unwrap_or(&0);
                        // .get returns Option<Ref<>
                    allocations.insert(user.clone(), current_alloc + volume);
                }  
                else { // partial fill
                    let current_alloc = *allocations.get(&user).as_deref().unwrap_or(&0);
                    allocations.insert(user.clone(), current_alloc + supply_);
                    let seq = buy_seq_no.load(Ordering::Relaxed);
                    bids.insert(
                        (Reverse(price), seq),
                        Bid::new(user.clone(), volume - supply_, price, seq)
                    );
                };
                break
            }
            Err(actual) => current = actual, 
                // other thread changed it, retry with new `current` value
        }
    } // end of CAS loop

    // metrics
    histogram!("buy_impl_duration_seconds")
        .record(start.elapsed().as_secs_f64());
    counter!("http_requests_total", "endpoint" => "buy").increment(1);
    gauge!("open_bids_count").set(bids.len() as f64);
    gauge!("supply_current").set(supply.load(Ordering::Relaxed) as f64);
}

/* 
curl -s -X POST localhost:8080/sell -H "Content-Type: application/json" -d "{\"volume\":500}"
*/
#[post("/sell")]
async fn sell(
    state: web::Data<AppState>, req: web::Json<SellRequest>
) -> impl Responder {
    if req.volume == 0 {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "sell_validation_failed",
            message: "volume must be greater than 0".to_string(),
            field: Some("volume"),
        });
    }

    let mut bids = ordered_locks_sell(&state);
    // instead of
        // let mut supply = state.supply.lock().unwrap();
        // let mut bids = state.bids.write().unwrap();

    sell_impl(
        &state.supply,
        &state.allocations,
        &mut bids,
        req.0);

    HttpResponse::Ok().finish()
}

/// Behavior: add supply and allocate to outstanding bids
/// 2. When sell comes, check stored list of bids and sell starting from the 
///    highest price or if no bids, store as supply.
///
///    Big O: N - retain will visit every elem (those returns are like breaks)
///
pub fn sell_impl( 
    supply: &AtomicU64,
    allocations: &DashMap<String, u64>,
    bids: &mut BTreeMap<PriceSeqPair, Bid>, 
    sell_req: SellRequest
) {
    //// add incoming sell into supply
    supply.fetch_add(sell_req.volume, Ordering::Relaxed);

    //// process/allocate outstanding bids; full or partial fill
    bids.retain(|_, bid| {
        let mut current = supply.load(Ordering::Acquire);
        loop { // CAS loop
            if current == 0 { return true; } // cannot fill, keep bid
            let new_val = current.saturating_sub(bid.volume);
                // if current >= volume { current - volume } // full fill 
                // else { 0 };                               // partial fill
            match supply.compare_exchange(
                current, new_val, Ordering::AcqRel, Ordering::Acquire
            ) {
                Ok(supply_) => { // we won, value updated, 
                                 // supply_ is NOT yet new_value, still current
                    let bid_user = bid.user.clone();
                    // full fill   : supply = 60, buy: 50 => supply: 10, bid: 50
                    if supply_ >= bid.volume { // full fill and remove bid
                        let alloc = 
                            *allocations.get(&bid_user).as_deref().unwrap_or(&0);
                        allocations.insert(bid_user.clone(), alloc + bid.volume);
                        break false // bid fully processed, remove bid
                    // partial fill: supply = 50, buy: 60 => supply:  0, bid: 10
                    } else { // partial fill and retain/keep bid
                        let alloc = 
                            *allocations.get(&bid_user).as_deref().unwrap_or(&0);
                        allocations.insert(bid_user, alloc + supply_);
                        bid.volume -= supply_;
                        break true
                    };
                }
                Err(actual) => current = actual, 
                    // other thread changed it, retry with new `current` value
            }
        } // end of CAS loop
    });

    let _total_alloc: u64 = allocations.iter().map(|e| *e).sum();
    // dbg!(_total_alloc);
}

/// Behavior: return the integer total VM-hours allocated to u1 so far.
/// Responses: 200 OK with body like 150, or appropriate 4xx on error 
/// (e.g., missing username).
/*
curl -s localhost:8080/allocation?username=u1
*/
fn allocation_impl(
    allocations: &DashMap<String, u64>, 
    req: AllocationQuery
) -> Result<u64> {
    allocations.get(&req.username).as_deref()
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
    let res = allocation_impl(&state.allocations, req.0);
    if let Ok(alloc) = res {
        Ok(alloc.to_string())
        // Debug:        
        // Ok( format!("\n{}: {alloc:?}\n", &req.username) + 
        //     &format!("\nstate: {state_:#?}\n ") )
    } else {
        Err(error::ErrorBadRequest("missing username\n"))
    }
}

/// debug: show full app state
pub async fn index(app_state: web::Data<AppState>) -> String {
    println!("-- thread: {:?}", std::thread::current().id());
    format!("state: {:#?}\n", app_state)
}

/// See `fn buy` for the alternative "JWT token feature flag" - pre API 
/// versioning solution.
/// See more in `tw_api_versioning.md` 
#[post("/v2/buy")]
async fn buy_v2(
    _state: web::Data<AppState>,
    _req: web::Json<BuyRequest>
) -> impl Responder {

    // buy_impl_v2()

    ">> server response: buy_v2".to_string()
}

/// See `src/tw_err_circuit_breaker.rs` for more
#[get("/btc-price")]
async fn btc_price(state: web::Data<AppState>) -> impl Responder {
    match state.price_feed.get_btc_price().await {
        Ok(price) => 
            HttpResponse::Ok().json(serde_json::json!({ "price": price })),
        // Fetch failed 3 times in a row
        Err(PriceFeedError::CircuitOpen) => 
            HttpResponse::ServiceUnavailable().json(serde_json::json!({
            "error": PriceFeedError::CircuitOpen.as_str(),
            "message": "price feed unavailable, try again later"
        })),
        // Fetch failed less then 3 times
        Err(e) => HttpResponse::BadGateway().json(serde_json::json!({
            "error": e.as_str()
        })),
    }
}


//// ----- Middleware

/// To be edited/used while debugging, 
/// also find/enable the line with "my_middleware" in tw_main_fn.rs
pub async fn my_middleware(
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

#[cfg(test)]
pub mod tests_lib {

    use super::*;

    // Helper fn
    pub fn buy_impl_for_test(state: &AppState, buy_req: BuyRequest) {
        let mut bids = ordered_locks_buy(state);
        buy_impl(
            &state.buy_seq_no,
            &state.supply,
            &state.allocations,
            &mut bids,
            buy_req
        );
    }

    // Helper fn
    pub fn sell_impl_for_test(state: &AppState, sell_req: SellRequest) {
        let mut bids = ordered_locks_sell(state);
        sell_impl(
            &state.supply,
            &state.allocations,
            &mut bids,
            sell_req
        );
    }

}


//// -----------  Integration tests using HTTP layer

#[cfg(test)]
mod http_tests {

    use actix_web::{http::StatusCode, test, test::TestRequest};
    use super::*;

    // Helper
    fn test_buy_request(req: BuyRequest) -> actix_http::Request {
        TestRequest::post().uri("/buy").set_json(req).to_request()
    }

    //Helper
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
        assert_eq!(
            state.bids.read().unwrap().values().next().unwrap().volume,
            50
        );
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
    use tests_lib::*;

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
                buy_impl_for_test(&state, BuyRequest::new("u1", 100, 2));
            })
        });

        for h in handles { h.await.unwrap() }

        let bids = &state.bids.read().unwrap();

        // 1. assert no same seq numbers
        let unique: HashSet<_> = bids.values().map(|b| b.seq).collect();
        assert_eq!(bids.values().len(), unique.len());

        // 2. assert ordering of same price bids - smallest seq no is first elem
        assert_eq!(bids.values().next().unwrap().seq, 1);
        // or
        // check two elem windows
        let seqs = bids.values().map(|b| b.seq).collect::<Vec<u64>>();
        assert!( seqs.windows(2).all(|w| w[1] > w[0]) );
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
                buy_impl_for_test(
                    &state, BuyRequest::new(format!("u{i}"), 10, 1)
                );
            }));
        }

        // 50 concurrent sells
        for _ in 0..50 {
            let state = state.clone();
            handles.push(tokio::spawn(async move {
                sell_impl_for_test(&state, SellRequest { volume: 10 });
            }));
        }

        for h in handles { h.await.unwrap(); }

        let total_alloc: u64 = state.allocations.iter().map(|e| *e).sum();
        let total_supply = 50 * 10; // 500
        assert_eq!(total_alloc + state.supply.load(Ordering::Relaxed), total_supply);
    }

    /// claude ai
    /// Buys and sells, check allocations + remaining supply = initial supply
    #[tokio::test]
    async fn buys_no_oversell_v2() {
        let state = web::Data::new(AppState::default());
    
        let mut handles = vec![];
        for i in 0..100 {
            let state = state.clone();
            handles.push(tokio::spawn(async move {
                buy_impl_for_test(
                    &state, BuyRequest::new(format!("u{i}"), 10, 1)
                );
            }));
        }
        for h in handles { h.await.unwrap(); }

        sell_impl_for_test(&state, SellRequest { volume: 500 });

        let total_alloc: u64 = state.allocations.iter().map(|e| *e).sum();
        assert_eq!(total_alloc + state.supply.load(Ordering::Relaxed), 500);
    }

    /// Buys and sells, check allocations + remaining supply = initial supply
    #[tokio::test]
    async fn buys_no_oversell() {
        let total_supply = 500;
        let state = web::Data::new(
            AppState { 
                supply: AtomicU64::new(total_supply), ..Default::default() 
            }
        );

        let handles = (0..100).map(|_| {
            let state = state.clone();
            tokio::spawn(async move {
                buy_impl_for_test(
                    &state, BuyRequest::new("u1", 200, 2)
                );
            })
        });

        for h in handles { h.await.unwrap(); }
    
        let total_alloc: u64 = state.allocations.iter().map(|e| *e).sum();
        let leftover_supply = state.supply.load(Ordering::Relaxed);
        assert_eq!(total_supply, total_alloc + leftover_supply);
    }

}


//// -----------  Property Tests

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;
    use tests_lib::*;
 
    proptest! {


    #[test]
    fn allocation_monotonically_increases(
        supplies in prop::collection::vec(1u64..10_000, 1..10),
            // 1–10 random elements, each between 1 and 10_000. 
            // e.g. [500, 3200, 77] or [9999] or [1, 200, 50, 8000]
        bids in prop::collection::vec((1u64..10_000, 1u64..100), 1..10),
            // e.g. [(100, 3), (5000, 7), (200, 1)], each tuple (volume, price)
    ) {
        let state = AppState::default();
        let mut prev_alloc = 0u64;

        // each time: 
        // - buy() : u1 bids
        // - sell(): supply becomes available, u1 gets allocation  
        // e.g.
        // bids    : [(100, 3), (5000, 7), (200, 1)], each tuple (volume, price)
        // supplies: [9999] 
        //
        for (volume, price) in bids {
            buy_impl_for_test(&state, BuyRequest::new("u1", volume, price));
            for supply in &supplies {
                sell_impl_for_test(&state, SellRequest { volume: *supply });
            }
            let alloc = *state.allocations.get("u1").as_deref().unwrap_or(&0);
            prop_assert!(alloc >= prev_alloc); // never decreases
            prev_alloc = alloc;
        }
    }

    /// supply added via /sell = total allocated + remaining supply 
    #[test]
    fn supply_conservation(
        // e.g. vec![1, 100, 5], vec![5, 9_000]
        supplies in prop::collection::vec(1u64..10_000, 1..10),
        // bid: (volume, price), e.g. vec![(100,1), (50,4)]
        bids in prop::collection::vec( (1u64..1_000, 1u64..100), 1..10 ),
    ) {
        let state = AppState::default();
        let total_supply: u64 = supplies.iter().sum();

        for (volume, price) in bids {
            buy_impl_for_test(&state, BuyRequest::new("u1", volume, price));
        }

        for supply in supplies {
            sell_impl_for_test(&state, SellRequest { volume: supply });
        }

        let total_alloc: u64 = state.allocations.iter().map(|e| *e).sum();
        prop_assert_eq!(
            total_supply,
            total_alloc + state.supply.load(Ordering::Relaxed)
        );
    }

    /// Early arrived request will be filled first for same price requests
    #[test]
    fn fifo_within_same_price(
        supply in 1u64..10_000,
        price in 1u64..50,
        volume in 1u64..1_000,
    ) {
        let state = AppState::default();
        buy_impl_for_test(&state, BuyRequest::new("u1", volume, price));
        buy_impl_for_test(&state, BuyRequest::new("u2", volume, price));
        sell_impl_for_test(&state, SellRequest { volume: supply });
        
        let u1_alloc = *state.allocations.get("u1").as_deref().unwrap_or(&0);
        let u2_alloc = *state.allocations.get("u2").as_deref().unwrap_or(&0);
    
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
        let state = AppState::default();
        buy_impl_for_test(&state, BuyRequest::new("lo", volume, lo_price));
        buy_impl_for_test(&state, BuyRequest::new("hi", volume, hi_price));
        sell_impl_for_test(&state, SellRequest { volume: supply });
        
        let lo_alloc = *state.allocations.get("lo").as_deref().unwrap_or(&0);
        let hi_alloc = *state.allocations.get("hi").as_deref().unwrap_or(&0);

        // hi should fill before lo
        prop_assert!(hi_alloc >= lo_alloc);
    }

    #[test]
    fn allocated_never_exceeds_supply(
        supply in 0u64..10_000,
        volume in 0u64..10_000,
        price in 1u64..100
    ) {
        let state = AppState { 
            supply: AtomicU64::new(supply), ..Default::default() 
        };
        buy_impl_for_test(&state, BuyRequest::new("u1", volume, price));
        let allocated = *state.allocations.get("u1").as_deref().unwrap_or(&0);
        prop_assert!(allocated <= supply);
    }

    #[test]
    fn partial_fill_remainder_stays_open(
        supply in 1u64..10_000, 
        volume in 2u64..10_000, 
        price in 1u64..100
    ) {
        prop_assume!(supply < volume); // force partial fill
        let state = AppState {
            supply: AtomicU64::new(supply), ..Default::default()
        };
        buy_impl_for_test(&state, BuyRequest::new("u1", volume, price));
        prop_assert_eq!(state.supply.load(Ordering::Relaxed), 0);
        let bids = state.bids.read().unwrap();
        prop_assert!(!bids.is_empty()); // remainder stays open
    }

    } // end of macro proptest!
}


//// -----------  Unit Tests

#[cfg(test)]
mod unit_tests {
    use actix_web::http::StatusCode;

    use super::*;

    use tests_lib::*;

    // Example from main spec. doc.
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
        let state = AppState::default();
        buy_impl_for_test(&state, BuyRequest::new("u1", 100, 3));
        buy_impl_for_test(&state, BuyRequest::new("u2", 150, 2));
        buy_impl_for_test(&state, BuyRequest::new("u3", 50, 4));
        sell_impl_for_test(&state, SellRequest { volume: 250 });
        assert_eq!(state.allocations.get("u3").as_deref().unwrap(), &50);
        assert_eq!(state.allocations.get("u1").as_deref().unwrap(), &100);
        assert_eq!(state.allocations.get("u2").as_deref().unwrap(), &100);
        let bids = state.bids.read().unwrap();
        let u2_bid = bids.get(&price_seq_pair(2, 2));
        assert_eq!(u2_bid.unwrap().volume, 50);
    }

    #[test]
    fn allocation() {
        let allocations = DashMap::new();
        allocations.insert("u1".to_string(), 100);    
        let state = AppState { allocations, ..Default::default() };

        // - good case
        let result = allocation_impl(
            &state.allocations, AllocationQuery { username: "u1".to_string() }
        ).unwrap();
        assert_eq!(result, 100);

        // - error case
        let result = allocation_impl(
            &state.allocations, AllocationQuery { username: "u2".to_string() }
        );
        let status = result.as_ref().unwrap_err().error_response().status();
        assert_eq!(status, StatusCode::BAD_REQUEST);
        let err_string = result.unwrap_err().to_string();
        assert_eq!(err_string, "missing username\n");
    }

    #[test]
    fn sell() {
        // 1. add incoming sell into supply
        let state = AppState::default();
        assert_eq!(state.supply.load(Ordering::Relaxed), 0);
        sell_impl_for_test(&state, SellRequest { volume: 400 });
        assert_eq!(state.supply.load(Ordering::Relaxed), 400);
    
        // 2. allocate outstanding bids
        // case: full fill - state.supply = 60, buy: 50 => supply: 10, bid: 50
        let state = AppState { 
            bids: RwLock::new( BTreeMap::from([
                ( price_seq_pair(2, 1), Bid::new("u1", 200, 2, 1) ) 
            ])),
            ..Default::default()
        };
        sell_impl_for_test(&state, SellRequest { volume: 300 });
        assert_eq!(state.allocations.get("u1").as_deref().unwrap(), &200);
        assert_eq!(state.supply.load(Ordering::Relaxed), 100);
        assert!(state.bids.read().unwrap().is_empty());
        // case: partial fill: state.supply = 50, buy: 60 => supply:  0, bid: 10
        let state = AppState { 
            bids: RwLock::new( BTreeMap::from([
                ( price_seq_pair(2, 1), Bid::new("u1", 100, 2, 1) ) 
            ])),
            ..Default::default()
        };
        sell_impl_for_test(&state, SellRequest { volume: 50 });
        assert_eq!(state.allocations.get("u1").as_deref().unwrap(), &50);
        assert_eq!(state.supply.load(Ordering::Relaxed), 0);
        let bids = state.bids.read().unwrap();
        let u1_bid = bids.get(&price_seq_pair(2, 1)).unwrap();
        assert_eq!(u1_bid.user, "u1");
        assert_eq!(u1_bid.volume, 50);
        assert_eq!(u1_bid.price, 2);
    }

    #[test]
    fn buy() {
        //// 1. sell immediately if there is unused supply
        // full fill
        let state = AppState { supply: AtomicU64::new(200), ..Default::default() };
        buy_impl_for_test(&state, BuyRequest::new("u1", 200, 2));
        assert_eq!(state.buy_seq_no.load(Ordering::Relaxed), 1);
        assert_eq!(state.allocations.get("u1").as_deref().unwrap(), &200);
        assert_eq!(state.supply.load(Ordering::Relaxed), 0);

        // partial fill
        let state = AppState { supply: AtomicU64::new(50), ..Default::default() };
        buy_impl_for_test(&state, BuyRequest::new("u1", 100, 2));
        assert_eq!(state.buy_seq_no.load(Ordering::Relaxed), 1);
        assert_eq!(state.supply.load(Ordering::Relaxed), 0);
        assert_eq!(state.allocations.get("u1").as_deref().unwrap(), &50);
        let bids = state.bids.read().unwrap();
        assert_eq!(bids.len(), 1);
        let u1_bid = bids.get(&price_seq_pair(2, 1)).unwrap();
        assert_eq!(u1_bid.volume, 50);
        assert_eq!(u1_bid.price, 2);
        assert_eq!(u1_bid.seq, 1);


        //// 2. otherwise, store req into bids
        let state = AppState::default();

        // case: basic first bid 
        buy_impl_for_test(&state, BuyRequest::new("u1", 100, 2));
        assert_eq!(state.buy_seq_no.load(Ordering::Relaxed), 1);
        let bids = state.bids.read().unwrap();
        assert_eq!(bids.len(), 1);
        let u1_bid = bids.get(&price_seq_pair(2, 1)).unwrap();
        assert_eq!(u1_bid.volume, 100);
        assert_eq!(u1_bid.price, 2);
        assert_eq!(u1_bid.seq, 1);
        drop(bids); // !! w/o this deadlock - 
                    // buy_impl_for_test will try to lock bids
        // case: earlier bids at the same price fill first
        buy_impl_for_test(&state, BuyRequest::new("u2", 100, 2));
        assert_eq!(state.buy_seq_no.load(Ordering::Relaxed), 2);
        let bids = state.bids.read().unwrap();
        assert_eq!(bids.len(), 2);
        let u2_bid = bids.get(&price_seq_pair(2, 2)).unwrap();
        assert_eq!(u2_bid.volume, 100);
        assert_eq!(u2_bid.price, 2);
        assert_eq!(u2_bid.seq, 2);
        let u1_bid = bids.get(&price_seq_pair(2, 1)).unwrap();
        assert_eq!(u1_bid.user, "u1");  // u1 bid first
        assert_eq!(u1_bid.seq, 1);
        drop(bids);
        // case: highest price always wins
        buy_impl_for_test(&state, BuyRequest::new("u3", 100, 3));
        assert_eq!(state.buy_seq_no.load(Ordering::Relaxed), 3);
        let bids = state.bids.read().unwrap();
        assert_eq!(bids.len(), 3);
        assert_eq!(bids.values().next().unwrap().user, "u3");
    }

    /// Higher price requests will be filled first
    #[test]
    fn higher_price_always_fills_first() {
        let state = AppState::default();
        buy_impl_for_test(&state, BuyRequest::new("u1", 200, 2));
        buy_impl_for_test(&state, BuyRequest::new("u2", 200, 4));
        buy_impl_for_test(&state, BuyRequest::new("u3", 200, 10));

        sell_impl_for_test(&state, SellRequest { volume: 200 });
        assert_eq!(state.allocations.get("u3").as_deref().unwrap(), &200);
        assert_eq!(state.allocations.get("u2").as_deref(), None);
        assert_eq!(state.allocations.get("u1").as_deref(), None);
        
        sell_impl_for_test(&state, SellRequest { volume: 200 });
        assert_eq!(state.allocations.get("u3").as_deref().unwrap(), &200);
        assert_eq!(state.allocations.get("u2").as_deref().unwrap(), &200);
        assert_eq!(state.allocations.get("u1").as_deref(), None);
    }

    /// Same user buys twice
    /// - allocation should accumulate, not overwrite
    /// - bids should be separate and unique
    #[test]
    fn buy_same_user_buys_twice() {
        //// 1. sell immediately if there is unused supply
        // full fill
        let state = AppState { supply: AtomicU64::new(400), ..Default::default() };
        buy_impl_for_test(&state, BuyRequest::new("u1", 200, 2));
        buy_impl_for_test(&state, BuyRequest::new("u1", 200, 2));
        assert_eq!(state.allocations.get("u1").as_deref().unwrap(), &400);
        // partial fill
        let state = AppState { supply: AtomicU64::new(300), ..Default::default() };
        buy_impl_for_test(&state, BuyRequest::new("u1", 200, 2));
        buy_impl_for_test(&state, BuyRequest::new("u1", 200, 2));
        assert_eq!(state.allocations.get("u1").as_deref().unwrap(), &300);

        //// 2. otherwise, store req into bids
        let state = AppState::default();
        buy_impl_for_test(&state, BuyRequest::new("u1", 100, 2));
        buy_impl_for_test(&state, BuyRequest::new("u1", 100, 2));
        let bids = state.bids.read().unwrap();
        assert_eq!(bids.len(), 2);
        let bid_1 = bids.get(&price_seq_pair(2, 1)).unwrap();
        let bid_2 = bids.get(&price_seq_pair(2, 2)).unwrap();
        assert_eq!(bid_1.user, "u1");
        assert_eq!(bid_2.user, "u1");
    }
}
