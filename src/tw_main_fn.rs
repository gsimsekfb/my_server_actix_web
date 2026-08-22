use actix_web::{App, HttpServer, web};
use tonic::transport::Server as GrpcServer;

use actix_hello::{tw_grpc, tw_main::*};

#[actix_web::main]
async fn main() -> std::io::Result<()> {

    if cfg!(not(feature = "disable_logs")) {
        println!("\n-- Server starting on localhost:8080 ...");
        println!("-- main's thread: {:?}", std::thread::current().id());
    }

    // Simple logger - see tw_debug.md for more
    // Note: should not be enabled w/ tracing spans feature
    if cfg!(
        all(not(feature = "disable_logs"), not(feature = "enable_tracing_spans"))
    ){
        env_logger::Builder::from_env(
            env_logger::Env::default().default_filter_or("info")
        ).init();
    }

    // For profiling w/ tracing spans - see tw_perf_testing.md for more
    // Note: should not be used when the env_logger is enabled
    if cfg!(feature = "enable_tracing_spans") {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::from_default_env()
                    .add_directive("info".parse().unwrap())
            )
            .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
            .init();
    }

    // web::Data<T> is struct Data<T>(Arc<T>)
    let app_state = web::Data::new( AppState::default() );
    let grpc_state = app_state.clone().into_inner(); // Arc<AppState>

    //// 1. HTTP server
    // closure will be run per worker thread (at startup), default workers: 8
    let http_server = HttpServer::new(move || { // move app_state into the closure
        App::new()
            // todo: add this into disable_logs feature, 
            // e.g. let app = ...; app.wrap(...);
            .wrap(actix_web::middleware::Logger::default())
            // clone for each worker thread
            .app_data(app_state.clone()) // register the created data
            .route("/", web::get().to(index))
            // todo: add this into disable_logs feature,
            // e.g. let app = ...; app.wrap(...);
            // todo: to be used while debugging
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

    // this section for timed stop 
    let http_server_handle = http_server.handle();
    let http_stop_handle = tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_mins(5)).await;
        http_server_handle.stop(true).await;
    });
    // the server itself
    let http_join_handle = tokio::spawn(http_server);

    //// 2. gRPC server (unchanged)
    let grpc_service = GrpcServer::builder()
        .add_service(tw_grpc::make_grpc_server(grpc_state))
        .add_service(tw_grpc::make_reflection_service());

    let grpc_handle = tokio::spawn(async move {
        grpc_service.serve("127.0.0.1:50051".parse().unwrap(),).await
    });

    // todo: keep both transports alive, rather than shutting down the process
    // if one fails (we do this now with select).
    //
    // Whichever finishes first (actix's own Ctrl+C handling, or the 5-min timer)
    // wins the race; abort the other task's handle directly
    let grpc_abort_handle = grpc_handle.abort_handle();
    tokio::select! {
        // !! 
        // Note that this isn't an assignment — it's select!'s branch syntax 
        // pattern = <async expression> => body, where http_join_handle 
        // (a JoinHandle, which implements Future) gets polled, and once it
        // resolves, its output (Result<T, JoinError>) is bound to the name res
        res = http_join_handle => {
            grpc_abort_handle.abort();
            if let Err(e) = res { eprintln!("HTTP server task panicked: {e}"); }
        }
        res = grpc_handle => {
            http_stop_handle.abort();
            if let Err(e) = res { eprintln!("gRPC server task panicked: {e}"); }
        }
    }

    println!("-- Server was shut-down after configured stop-timeout");
    std::io::Result::Ok(())
}
