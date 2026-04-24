#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(unused_variables)]
#![allow(clippy::items_after_test_module)]

use actix_web::{HttpResponse, Responder, get, web};
use std::sync::Mutex;
use serde::{Deserialize, Serialize};

// Topics:
// 1. Integration tests (aka test through HTTP layer - production standard)
// 2. Unit tests (direct handler func call w/o HTTP layer)
//
// - Integration test your handlers
// - Unit test your business logic 


#[derive(Debug, Deserialize, Serialize)]
struct AppState {
    counter: Mutex<i32>,
        // Arc will come from web::Data, see below
}

// This handler runs per request, as a tokio task on one of the worker threads
#[get("/")]
async fn index(data: web::Data<AppState>) -> impl Responder {
    println!("-- thread: {:?}", std::thread::current().id());

    let mut counter = data.counter.lock().unwrap(); // get counter's MutexGuard
    *counter += 1; // access counter inside MutexGuard

    HttpResponse::Ok().json(*counter)
}

// ** No routing (#[get("/")]), in order to be able to be called as an fn 
//    by unit tests.
//    Note that adding routing macro (e.g. get) will convert this fn to struct 
async fn index_2(data: web::Data<AppState>) -> HttpResponse {
    println!("-- thread: {:?}", std::thread::current().id());

    let mut counter = data.counter.lock().unwrap(); // get counter's MutexGuard
    *counter += 1; // access counter inside MutexGuard

    if *counter > 99 {
        return HttpResponse::TooManyRequests().finish();
    }

    HttpResponse::Ok().json(*counter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{App, http::{self, header::ContentType}, test, web};

    //// 1. Integration tests:
    //    (aka test through HTTP layer - production standard)

    // i. Success cases
    #[actix_web::test]
    async fn integration_test_index_get() {
        let app_state = web::Data::new(AppState { counter: Mutex::new(0) });
        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(index),
        )
        .await;

        // a. basic service working test 
        let req = test::TestRequest::default() // default: Get /
            .insert_header(ContentType::plaintext())
            .to_request();
        let resp = test::call_service(&app, req).await; // ServiceResponse
        assert!(resp.status().is_success());

        // b. basic service returned data test
        let req = test::TestRequest::get().uri("/").to_request();
        let resp: i32 = test::call_and_read_body_json(&app, req).await;
        let counter = resp;
        assert_eq!(counter, 2);
    }

    /// ii. Error case, post service is not active
    #[actix_web::test]
    async fn integration_test_index_post() {
        let app_state = web::Data::new(AppState { counter: Mutex::new(0) });
        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(index)
                // .service(post_)  // case_2: post enabled
        ).await;

        // a. basic service working test 
        let req = test::TestRequest::post().uri("/post").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_client_error());
        // assert!(resp.status().is_success());  // case_2
    }

    //// 2. Unit tests:
    ////    - uses direct function call, does not use routing
    ////    - has pretty limited value for applications 
    
    #[actix_web::test]
    async fn unit_test_index_ok() {
        let app_state = web::Data::new(AppState { counter: Mutex::new(0) });
        // ** direct function call, no routing
        let resp = index_2(app_state).await; 
        assert_eq!(resp.status(), http::StatusCode::OK);
    }

    #[actix_web::test]
    async fn unit_test_index_should_fail() {
        let app_state = web::Data::new(AppState { counter: Mutex::new(99) });
        // ** direct function call, no routing
        let resp = index_2(app_state).await; 
        assert_eq!(resp.status(), http::StatusCode::TOO_MANY_REQUESTS);
    }
}


fn main(){}
//// Not needed, use for debugging
/* 
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("-- Server running on localhost:8080 ...");
    println!("-- main's thread: {:?}", std::thread::current().id());

    // Note: web::Data created _outside_ HttpServer::new closure
    // which makes it global shared state
    // - web::Data<T> is struct Data<T>(Arc<T>) — so the pointer is 
    // shared safely across threads
    let app_state = web::Data::new(AppState { counter: Mutex::new(0) });

    // closure will be run per worker thread (at startup), default workers: 8
    HttpServer::new(move || { // move app_state into the closure
        App::new()
            // clone for each worker thread
            .app_data(app_state.clone()) // register the created data
            .service(index)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await?;

    println!("-- Server was shut-down");
    std::io::Result::Ok(())
}
 */
