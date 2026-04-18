#![allow(non_camel_case_types)]
#![allow(dead_code)]

//// Topic: Error Logging (application level)
// Error logging — manual, specific errors
// 
// in this file this line inside index() handler:
//     info!("{}", err);
//
// see also Logging MiddlewareError which logs every HTTP request/response
// automatically about what happened (request level)

use actix_web::{error, get, middleware::Logger, App, HttpServer, Result};
use derive_more::derive::{Display, Error};
use log::info;

#[derive(Debug, Display, Error)]
#[display("my error: {name}")]
pub struct MyError {
    name: &'static str,
}

// Use default implementation for `error_response()` method
impl error::ResponseError for MyError {}

#[get("/")]
async fn index() -> Result<&'static str, MyError> {
    let err = MyError { name: "test error" };
    info!("{}", err); // **
    Err(err)
}

#[rustfmt::skip]
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    unsafe {
        std::env::set_var("RUST_LOG", "info");
        std::env::set_var("RUST_BACKTRACE", "1");
    }
    env_logger::init();

    HttpServer::new(|| {
        let logger = Logger::default();

        App::new()
            .wrap(logger)
            .service(index)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

// curl -is localhost:8080
    // HTTP/1.1 500 Internal Server Error
    // date: Fri, 17 Apr 2026 12:34:35 GMT
    // my error: test error

// Logs:
// [2026-04-17T12:34:36Z INFO  main] my error: test error
// [2026-04-17T12:34:36Z INFO  actix_web::middleware::logger] 127.0.0.1 "GET / HTTP/1.1" 500 20 "-" "curl/8.18.0" 0.001845
