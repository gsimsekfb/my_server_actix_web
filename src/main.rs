use actix_web::{App, HttpServer, dev::Service as _, web};
use futures_util::future::FutureExt;

// A. inline middleware, good for prototyping
// using wrap_fn:

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            // App-level middleware
            .wrap_fn(|req, srv| { // 1. Request arrives → wrap_fn closure runs
                // 2. Before handler
                println!("MW: pre-handler running... You requested: {}", req.path());
                srv.call(req).map(|res| { // 3. calls the handler, see below
                    println!("MW: post-handler running..."); // 4. After handler 
                    res
                })
            })
            .route(
                "/index.html",
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

async fn my_middleware(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    // pre-processing
    next.call(req).await
    // post-processing
}

// #[actix_web::main]
async fn main_() {
    let app = App::new().wrap(from_fn(my_middleware));
}

