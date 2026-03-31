use actix_web::{get, web, App, HttpServer, Result};
use serde::Deserialize;

// Topic: extract data from URL (e.g. "localhost:8080/users/123/alex")
// sub topics:
// - 1. w/ using (u32, String)
// - 2. w/ using struct 

//// 1.
/// extract path info from "/users/{user_id}/{friend}" url
/// {user_id} - deserializes to a u32
/// {friend} - deserializes to a String
///
#[get("/users/{user_id}/{friend}")] // define path parameters
async fn index(path: web::Path<(u32, String)>) -> Result<String> {
    let (user_id, friend) = path.into_inner();
    Ok(format!("Welcome {}, user_id {}!", friend, user_id))
}
    // curl -s "localhost:8080/users/123/alex"
    // Welcome alex, user_id 123!

// or
//// 2. using struct
#[derive(Deserialize)] // needed for actix to deserialize URL params into struct
struct Info {
    user_id: u32,
    friend: String,
}

/// extract path info using serde
#[get("/users/{user_id}/{friend}")] // define path parameters
async fn index_v2(info: web::Path<Info>) -> Result<String> {
    Ok( format!("Welcome-v2 {}, user_id {}!", info.friend, info.user_id) )
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new()
        .service(index_v2))
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}
