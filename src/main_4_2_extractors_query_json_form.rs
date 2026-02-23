use actix_web::{get, post, web, App, HttpServer, Result};
use serde::Deserialize;

#[derive(Deserialize)]
struct Info {
    username: String,
    id: i32
}

//// 1. Query

// this handler gets called if the query deserializes into `Info` successfully
// otherwise a 400 Bad Request error response is returned
#[get("/")]
async fn index(info: web::Query<Info>) -> String {
    format!("Welcome {}, id:{}", info.username, info.id)
}
    // curl -s "localhost:8080/?username=alex&id=42"
    // Welcome alex!
    // curl -s "localhost:8080"
    // Query deserialize error: missing field `username`


//// 2. JSON

/// deserialize `Info` from request's body
#[post("/submit")]
async fn submit(info: web::Json<Info>) -> Result<String> {
    Ok(format!("Welcome {}, id:{} !", info.username, info.id))
}
    // curl -X POST http://localhost:8080/submit -H "Content-Type: application/json" -d "{\"username\":\"alice\",\"id\":42}"
    // Welcome alice, id:42


//// 3. Form

#[derive(Deserialize)]
struct FormData {
    username: String,
}
    
/// extract form data using serde
/// this handler gets called only if the content type is *x-www-form-urlencoded*
/// and the content of the request could be deserialized to a `FormData` struct
#[post("/submit-2")]
async fn submit_2(form: web::Form<FormData>) -> Result<String> {
    Ok(format!("Welcome {}!", form.username))
}
    // curl -X POST http://localhost:8080/submit-2 -d "username=Alex"
        // -d: set `Content-Type` to `application/x-www-form-urlencoded` 
    // Welcome Alex!
    // or same:
    // curl -X POST http://localhost:8080/submit-2 -H "Content-Type: application/x-www-form-urlencoded" -d "username=YourName"

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new()
        .service(index)
        .service(submit)
        .service(submit_2)
    )
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
