use actix_web::{get, web, App, HttpServer};

// Topics:
// Threads, tasks
// State that is local per worker thread

struct AppState {
    app_name: String,
}

// This handler runs per request, as a tokio task on one of the worker threads
#[get("/")]
async fn index(data: web::Data<AppState>) -> String {
    println!("-- thread: {:?}", std::thread::current().id());

    let app_name = &data.app_name;
    format!("Hello {app_name}!")    // response
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("-- Server running on localhost:8080 ...");
    println!("-- main's thread: {:?}", std::thread::current().id());

    // How many worker threads:
    // By default actix-web spawns one worker thread per CPU core. 
    // # of cores, which matches the default worker count.
    println!("workers: {}", std::thread::available_parallelism().unwrap());
        // workers: 8

    // Note: State is "local per worker thread", not shared between workers
    // - closure will be run per worker thread (at startup)
    HttpServer::new(|| {
        App::new()
            // web::Data is Data<T>(Arc<T>) — so the pointer is 
            // shared safely across threads
            .app_data(
                web::Data::new( AppState{ app_name: String::from("Actix Web")} )
            )
            .service(index)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await?;

    println!("Server was shut-down");
    std::io::Result::Ok(())
}