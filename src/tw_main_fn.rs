use actix_web::{App, HttpServer, web};

use actix_hello::tw_main::*;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("\n-- Server starting on localhost:8080 ...");
    println!("-- main's thread: {:?}", std::thread::current().id());

    //// DO NOT use both of them at the same time: 
    // 1. simple logger
    // env_logger::init();
    // 2. for profiling w/ tracing spans - see tw_perf_testing.md for more
    // tracing_subscriber::fmt()
    //     .with_env_filter(
    //         tracing_subscriber::EnvFilter::from_default_env()
    //             .add_directive("info".parse().unwrap())
    //     )
    //     .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
    //     .init();

    // web::Data<T> is struct Data<T>(Arc<T>)
    let app_state = web::Data::new( AppState::default() );

    // closure will be run per worker thread (at startup), default workers: 8
    let server = HttpServer::new(move || { // move app_state into the closure
        App::new()
            // .wrap(actix_web::middleware::Logger::default())
            // clone for each worker thread
            .app_data(app_state.clone()) // register the created data
            .route("/", web::get().to(index))
            // .wrap(actix_web::middleware::from_fn(my_middleware))
            .service(sell)
            .service(buy)
            .service(allocation)
            .service(buy_v2)
            .service(btc_price)
    })
    .workers(2) // to have a lite program
    .bind(("127.0.0.1", 8080))?
    .run();
 
    let handle = server.handle();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_mins(5)).await;
        handle.stop(true).await;
    });
    server.await?;

    println!("-- Server was shut-down after configured stop-timeout");
    std::io::Result::Ok(())
}
