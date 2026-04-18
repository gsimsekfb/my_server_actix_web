#![allow(non_camel_case_types)]
#![allow(dead_code)]

// 1. Using default impl. (500 - internal server error) of ResponseError
// 2. Quick in place conversion with error helpers 
// 3. Custom impl. of ResponseError 
// 4. Separation of User and Non-User facing errors


//// 1. Using default impl. (500 - internal server error) of ResponseError

use actix_web::{error, get, Result};
use derive_more::derive::{Display, Error};
#[derive(Debug, Display, Error)]
#[display("my error: {name}")]
struct MyError {
    name: &'static str,
}

// ** Use default implementation for `error_response()` method
impl error::ResponseError for MyError {}
    // ResponseError has a default implementation for error_response() 
    // that will render a 500 (internal server error)
    // status code 500, INTERNAL_SERVER_ERROR

#[get("/1")]
async fn index_1() -> Result<&'static str, MyError> {
    Err(MyError { name: "test" })
}
// curl -is localhost:8080/1
    // HTTP/1.1 500 Internal Server Error
    // date: Fri, 17 Apr 2026 09:45:59 GMT
    //
    // my error: test



//// 2. Quick in place conversion with error helpers 

#[derive(Debug)]
struct MyError_2 {
    name: &'static str,
}

// About "Result<String>":
// from actix_web::error files:
// pub type Result<T, E = Error> = std::result::Result<T, E>;
    // Error: actix_web::Error
    // meaning: Let's use Result<T, E> instead of std::result::Result<T, E>, 
    // and E type param will be actix_web::Error if user not specifies.
// in short actix_web::Result<String> is std::Result<String, actix_web::Error>
#[get("/2")]
async fn index_2() -> Result<String> { 
    let result = Err(MyError_2 { name: "test error" });
    result.map_err(|err| error::ErrorBadRequest(err.name)) // **
        // bound to http status code 400 BAD_REQUEST
}
// curl -is localhost:8080/2
    // HTTP/1.1 400 Bad Request
    // date: Fri, 17 Apr 2026 09:47:09 GMT
    //
    // test error




//// 3. Custom impl. of ResponseError
////    all error mapping are in one place (in fn status_code) (as oppose to ex 2)

use actix_web::{
    http::{header::ContentType, StatusCode},
    HttpResponse,
};

#[derive(Debug, Display, Error)]
enum MyError_3 {
    #[display("internal error")]
    InternalError,

    #[display("bad request")]
    BadClientData,

    #[display("timeout")]
    Timeout,
}

impl error::ResponseError for MyError_3 {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::html())
            .body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        match *self {
            MyError_3::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
            MyError_3::BadClientData => StatusCode::BAD_REQUEST,
            MyError_3::Timeout => StatusCode::GATEWAY_TIMEOUT,
        }
    }
}

#[get("/3")]
async fn index_3() -> Result<&'static str, MyError_3> {
    Err(MyError_3::BadClientData)
}
// curl -is localhost:8080/3
//     HTTP/1.1 400 Bad Request
//     date: Fri, 17 Apr 2026 11:49:14 GMT

//     bad request



#[derive(Debug, Display, Error)]
enum UserError {
    #[display("Validation error on field: {field}")]
    ValidationError { field: String },
}

impl error::ResponseError for UserError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::html())
            .body(self.to_string())
    }
    fn status_code(&self) -> StatusCode {
        match *self {
            UserError::ValidationError { .. } => StatusCode::BAD_REQUEST,
        }
    }
}


//// 4. Separation of User and Non-User facing errors

/* 
// 1. INTERNAL errors (all possible failures, never shown to user)
#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("DB error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Redis error: {0}")]
    Cache(#[from] redis::RedisError),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// 2. USER-FACING errors (what client actually sees)
#[derive(Debug, Display, Error)]
enum ApiError {
    #[display("Validation error: {field}")]
    Validation { field: String },      // 400 - their fault

    #[display("Not found")]
    NotFound,                          // 404

    #[display("An internal error occurred. Please try again later.")]
    Internal,                          // 500 - our fault, hide details
}

impl ResponseError for ApiError { ... }

// CONVERSION — bridge between them
impl From<AppError> for ApiError {
    fn from(err: AppError) -> Self {
        // log the real error here
        tracing::error!("Internal error: {:?}", err);
        
        ApiError::Internal  // user only sees this
    }
}

// HANDLER — clean usage
async fn get_user(id: web::Path<u32>) -> Result<HttpResponse, ApiError> {
    let user = db.find(id)
        .await
        .map_err(AppError::Database)?  // AppError
        .map_err(ApiError::from)?;     // → ApiError (logged, hidden)

    Ok(HttpResponse::Ok().json(user))
}

*/



 // ========== main



use actix_web::{App, HttpServer};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("\nServer running on localhost:8080 ...");

    let server = HttpServer::new(|| {
        App::new()
            .service(index_1)
            .service(index_2)
            .service(index_3)
    })
    .bind(("127.0.0.1", 8080))?
    .run();

    let handle = server.handle();

    // Spawn shutdown task
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        handle.stop(true).await; // true = graceful shutdown
    });

    server.await?;    
 
    println!("Server was shut-down");
    std::io::Result::Ok(())
}
/* 
curl -s localhost:8080
my error: test
 */