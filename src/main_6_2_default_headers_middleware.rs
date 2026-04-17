use actix_web::{get, App, HttpResponse, HttpServer, Responder};
// Default headers middleware:
use actix_web::middleware::DefaultHeaders;


//// Topics
// - Default headers middleware
// In production, DefaultHeaders is used mostly for security headers 
//
// Security headers     → protect every response (most common use)
// Metadata headers     → versioning, tracing, environment info


#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            // Added to ALL responses
            .wrap(DefaultHeaders::new().add(("App-Version", "1.1")))
            .service(hello)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
// Headers + Body
    // curl -i "localhost:8080"
        // HTTP/1.1 200 OK
        // content-length: 12
        // app-version: 1.0
        // date: Fri, 13 Feb 2026 08:39:24 GMT
        //
        // Hello world!

// Headers only
    // curl -I "localhost:8080"
        // HTTP/1.1 404 Not Found
        // content-length: 0
        // app-version: 1.1
        // date: Fri, 13 Feb 2026 09:06:53 GMT
