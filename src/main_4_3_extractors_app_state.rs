use actix_web::{get, web, App, HttpServer, Responder};
use std::{
    cell::Cell,
    sync::atomic::{AtomicUsize, Ordering},
    sync::Arc,
};

// Topics: app state extracting, state per thread, shared state between workers
//
// Actix-web extracts web::Data<AppState> from the app's registered data 
// and passes it to your handler. You don't manually look it up — actix does 
// it for you based on the parameter type.
//
// Same pattern for all extractors:
// 
// async fn handler(
//     path: web::Path<Info>,        // extracted from URL
//     query: web::Query<Info>,      // extracted from query string
//     body: web::Json<Info>,        // extracted from request body
//     state: web::Data<AppState>,   // extracted from app state
// )

#[derive(Clone)] // needed, HttpServer::new requires clonable closure, which
                 // means that everything closure owns must be Clone
struct AppState {
    local_count: Cell<usize>, // per-worker thread, each worker gets its own
    global_count: Arc<AtomicUsize>, // shared across all workers
}

#[get("/")]
async fn show_counters(app_state: web::Data<AppState>) -> impl Responder {
    print!("-- thread: {:?}", std::thread::current().id());

    let str = format!(
        " - global_count: {} - local_count: {}",
        app_state.global_count.load(Ordering::Relaxed),
        app_state.local_count.get()
    );
    println!("{str}");

    str
}

#[get("/add")]
async fn incr_counters(data: web::Data<AppState>) -> impl Responder {
    print!("-- thread: {:?}", std::thread::current().id());
    data.global_count.fetch_add(1, Ordering::Relaxed);

    let local_count = data.local_count.get();
    data.local_count.set(local_count + 1);

    let str = format!(
        " - global_count: {} - local_count: {}",
        data.global_count.load(Ordering::Relaxed),
        data.local_count.get()
    );
    println!("{str}");

    str
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("-- main's thread: {:?}", std::thread::current().id());

    let app_state = AppState {
        local_count: Cell::new(0),
        global_count: Arc::new(AtomicUsize::new(0)),
    };

    HttpServer::new(move || { // move: takes ownership of app_state
        App::new()
            // clone app_state for each worker thread started
            //   - local_count: Cell cloned → each worker gets its own counter
            //   - global_count: Arc cloned → all workers share same AtomicUsize
            .app_data(web::Data::new(app_state.clone()))
            .service(show_counters)
            .service(incr_counters)
    })
    .workers(4)
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

// output:
//    -- main's thread: ThreadId(1)
// 1. -- thread: ThreadId(2)  - global_count: 0 - local_count: 0
// 2. -- thread: ThreadId(3)  - global_count: 0 - local_count: 0
// 3. -- thread: ThreadId(4)  - global_count: 0 - local_count: 0
// 4. -- thread: ThreadId(5)  - global_count: 0 - local_count: 0
// 5. -- thread: ThreadId(2)  - global_count: 0 - local_count: 0
// 6. -- thread: ThreadId(3)  - global_count: 1 - local_count: 1
// 7. -- thread: ThreadId(4)  - global_count: 2 - local_count: 1
// 8. -- thread: ThreadId(5)  - global_count: 3 - local_count: 1
// 9. -- thread: ThreadId(2)  - global_count: 4 - local_count: 1
// 10. -- thread: ThreadId(3) - global_count: 5 - local_count: 2
// 11. -- thread: ThreadId(4) - global_count: 6 - local_count: 2
// 12. -- thread: ThreadId(5) - global_count: 6 - local_count: 1

// 1. curl -s "localhost:8080"      - global_count: 0 - local_count: 0"
// 2. curl -s "localhost:8080"      - global_count: 0 - local_count: 0"
// 3. curl -s "localhost:8080"      - global_count: 0 - local_count: 0"
// 4. curl -s "localhost:8080"      - global_count: 0 - local_count: 0"
// 5. curl -s "localhost:8080"      - global_count: 0 - local_count: 0"
// 6. curl -s "localhost:8080/add"  - global_count: 1 - local_count: 1"
// 7. curl -s "localhost:8080/add"  - global_count: 2 - local_count: 1"
// 8. curl -s "localhost:8080/add"  - global_count: 3 - local_count: 1"
// 9. curl -s "localhost:8080/add"  - global_count: 4 - local_count: 1"
// 10. curl -s "localhost:8080/add" - global_count: 5 - local_count: 2"
// 11. curl -s "localhost:8080/add" - global_count: 6 - local_count: 2"
// 12. curl -s "localhost:8080"     - global_count: 6 - local_count: 1"
