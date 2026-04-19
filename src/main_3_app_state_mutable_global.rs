use actix_web::{web, App, HttpServer};
use std::sync::Mutex;

// Topics:
// Threads, tasks
// State that is globally shared

struct AppState {
    counter: Mutex<i32>, // Mutex is necessary to mutate safely across threads
        // Arc will come from web::Data, see below
}

// This handler runs per request, as a tokio task on one of the worker threads
async fn index(data: web::Data<AppState>) -> String {
    println!("-- thread: {:?}", std::thread::current().id());

    let mut counter = data.counter.lock().unwrap(); // get counter's MutexGuard
    *counter += 1; // access counter inside MutexGuard

    format!("Request number: {counter}") // response with count
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("-- Server running on localhost:8080 ...");
    println!("-- main's thread: {:?}", std::thread::current().id());

    // Note: web::Data created _outside_ HttpServer::new closure
    // which makes it global shared state
    // - web::Data<T> is struct Data<T>(Arc<T>) — so the pointer is 
    // shared safely across threads
    let app_state = web::Data::new(
        AppState { counter: Mutex::new(0) }
    );

    // closure will be run per worker thread (at startup), default workers: 8
    HttpServer::new(move || { // move app_state into the closure
        App::new()
            // clone for each worker thread
            .app_data(app_state.clone()) // register the created data
            .route("/", web::get().to(index))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await?;

    println!("-- Server was shut-down");
    std::io::Result::Ok(())
}