use actix_web::body::BoxBody;
use actix_web::{App, Either, Error, HttpRequest, HttpResponse, HttpServer, Responder, get, http, web};
use futures::StreamExt;

// Src:
// https://actix.rs/docs/handlers/


// 1-a. Actix Web provides Responder impls for some types:
// By default Actix Web provides Responder implementations for some standard 
// types, such as &'static str, String, etc.
// Full list: 
// https://docs.rs/actix-web/4.12.1/actix_web/trait.Responder.html#foreign-impls

// Note that no need to return "impl Responder"
#[get("/index")]
async fn index(_req: HttpRequest) -> &'static str {
    "Hello world!"
}

#[get("/index_")]
async fn index_(_req: HttpRequest) -> String {
    "Hello world 2!".to_owned()
}

// You can also change the signature to return impl Responder which works well 
// if more complex types are involved.

#[get("/index__")]
async fn index__(_req: HttpRequest) -> impl Responder {
    web::Bytes::from_static(b"Hello world! 3")
}


// 1-b. Response with custom type

use serde::Serialize;

#[derive(Serialize)]
struct MyObj {
    name: &'static str,
}

impl Responder for MyObj {
    type Body = BoxBody;

    fn respond_to(self, _req: &actix_web::HttpRequest) -> HttpResponse<Self::Body> {
        let body = serde_json::to_string(&self).unwrap();

        // Create response and set content type
        HttpResponse::Ok()
            .content_type(http::header::ContentType::json())
            .body(body)
    }
}

#[get("/myobj")]
async fn my_obj() -> impl Responder { MyObj { name: "myobj-1" } }



// 2. Streaming response body

#[get("/stream")]
async fn stream() -> HttpResponse {
    let big_data = vec!["chunk_1\n", "chunk_2\n", "chunk_N\n"];
    let stream = futures::stream::iter(big_data)
        .map(|chunk| Ok::<_, Error>(web::Bytes::from(chunk)));

    HttpResponse::Ok().content_type("application/json").streaming(stream)
}
    // curl -s "localhost:8080/stream"
    // chunk_1
    // chunk_2
    // chunk_N



// 3. Different return types (Either)

use serde::Deserialize;

#[derive(Deserialize)]
struct Flags {
    flag_1: bool,
}

type RegisterResult = Either<HttpResponse, Result<&'static str, actix_web::Error>>;

#[get("/flags")]
async fn ww(query: web::Query<Flags>) -> RegisterResult {
    if query.flag_1 {
        Either::Right(Ok("flag_1 is true"))
    } else {
        Either::Left(HttpResponse::BadRequest().body("Bad data"))
    }
}
    // curl -s "localhost:8080/flags?flag_1=true"
    // flag_1 is true
    // 
    // curl -s "localhost:8080/flags?flag_1=false"
    // Bad data


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| { 
        App::new()
            .service(index)
            .service(index_)
            .service(index__)
            .service(stream)
            .service(ww)
            .service(my_obj)
        })
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}