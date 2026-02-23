use actix_web::{get, web, App, HttpServer, Result};
use serde::Deserialize;

//// 1.
/// extract path info from "/users/{user_id}/{friend}" url
/// {user_id} - deserializes to a u32
/// {friend} - deserializes to a String
#[get("/users/{user_id}/{friend}")] // <- define path parameters
async fn index(path: web::Path<(u32, String)>) -> Result<String> {
    let (user_id, friend) = path.into_inner();
    Ok(format!("Welcome {}, user_id {}!", friend, user_id))
}
    // curl -s "localhost:8080/users/11/alex"
    // Welcome alex, user_id 11!

// or
//// 2. with struct
#[derive(Deserialize)]
struct Info {
    user_id: u32,
    friend: String,
}

/// extract path info using serde
#[get("/users/{user_id}/{friend}")] // <- define path parameters
async fn index_v2(info: web::Path<Info>) -> Result<String> {
    Ok(format!(
        "Welcome-v2 {}, user_id {}!", info.friend, info.user_id
    ))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new()
        .service(index_v2))
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}
