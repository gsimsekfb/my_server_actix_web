use actix_web::{App, HttpServer, dev::Service as _, web};
use futures_util::future::FutureExt;

//// Topics: 
// Middleware:
// - A layer that wraps request/response and can block, enrich or transform them 
// - "One" use of it: 
//   can run before and after handler:
//   Request → middleware (pre) → handler → middleware (post) → Response
//
// A. inline middleware (good for prototyping) using wrap_fn
// B. standalone middleware fn using from_fn:



// A. inline middleware (good for prototyping) using wrap_fn:

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            // App-level middleware
            // 1. Request arrives → wrap_fn closure runs (before handler)
            .wrap_fn(|req, srv| { 
                println!("MW: pre-handler running... You requested: {}", req.path());
                // 2. calls the handler (handler below)
                srv.call(req).map(|res| { 
                    println!("MW: post-handler running..."); // 4. After handler 
                    res
                })
            })
            .route("/index.html",
                // 3. Handler
                web::get().to(|| async { "Handler: business logic" }), 
                // 5. Response sent to client
            )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}


// or 
// B. using from_fn:

use actix_web::{
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    middleware::{from_fn, Next},
    Error,
};

// Full picture:
// 1. Request arrives
// 2. my_middleware (pre-processing)
// 3. next.call(req) calls handler: || async { "Handler: business logic" }
// 4. middleware (post-processing)
// 5. Response sent to client

async fn my_middleware(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    // step-1. pre-processing
    // ...

    next.call(req).await

    // step-3. post-processing
    // ...
}

// #[actix_web::main]
#[allow(dead_code)]
async fn main_() {
    let _app = App::new()
        .route("/index.html",
            // step-2. Handler
            web::get().to(|| async { "Handler: business logic" }), 
        )
        .wrap(from_fn(my_middleware)
    );
}

