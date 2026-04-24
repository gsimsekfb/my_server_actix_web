use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};


//// Topics:
// Basics: path, route, service, handler
// get/post handlers


//// Basics: path, route, service, handler
// path:  /
// route: GET /
// service: route + handler
// handler: this fn
// !! Note that adding routing macro (e.g. get) will convert this fn to struct 
#[get("/")]
async fn root_path() -> impl Responder {
    HttpResponse::Ok().body("hi from root/index path")
}

#[post("/echo")]
async fn echo(req_body: String) -> impl Responder {
    dbg!(&req_body);
    HttpResponse::Ok().body(req_body)
}

async fn manual_hi() -> impl Responder {
    HttpResponse::Ok().body("Hi there!")
}

// --------------------------------------------------------
//// 1. GETs
// curl -s localhost:8080   // !! default is GET
// hi from root/index path

// curl -s localhost:8080/hi
// Hi there! 

//// 2. POST
// curl -s localhost:8080/echo -d "aaa" 
// aaa
    // !! -d "this": POST/send this as req body 
// --------------------------------------------------------

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Server running on localhost:8080 ...");

    HttpServer::new(|| {
        App::new()
            .service(root_path)
            .service(echo)
            .route("/hi", web::get().to(manual_hi))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await?;
 
    println!("Server was shut-down");
    std::io::Result::Ok(())
}
