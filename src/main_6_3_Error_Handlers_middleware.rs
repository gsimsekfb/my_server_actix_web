use actix_web::middleware::{ErrorHandlerResponse, ErrorHandlers};
use actix_web::{
    dev,
    http::{header, StatusCode},
    web, App, HttpResponse, HttpServer, Result,
};

//// Topic
// Error Handler Middleware:
// how to intercept specific HTTP error responses (like 500) and modify them 
// before they reach the client.

// Src:
// https://actix.rs/docs/middleware#error-handlers

fn add_error_header<B>(mut res: dev::ServiceResponse<B>) -> Result<ErrorHandlerResponse<B>> {
    res.response_mut().headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("Error"), // Change header
    );

    Ok(ErrorHandlerResponse::Response(res.map_into_left_body()))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .wrap( // Intercept any 500 response and modify it
                ErrorHandlers::new()
                    .handler(StatusCode::INTERNAL_SERVER_ERROR, add_error_header),
            )
            // Always return 500 status
            .service(web::resource("/").route(
                web::get().to(HttpResponse::InternalServerError))
            )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
// ** w Error handler middleware:
    // curl -i "localhost:8080"
        // HTTP/1.1 500 Internal Server Error
        // content-length: 0
        // content-type: Error   // <-- added by mw
        // date: Fri, 13 Feb 2026 13:00:32 GMT

// ** w/o Error handler middleware:
    // curl -i "localhost:8080"
        // HTTP/1.1 500 Internal Server Error
        // content-length: 0
        // date: Fri, 13 Feb 2026 13:01:10 GMT
