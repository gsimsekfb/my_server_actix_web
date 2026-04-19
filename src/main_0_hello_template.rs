use actix_web::{get, App, HttpResponse, HttpServer, Responder};


#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok().body("hi from root/index path")
}


// --------------------------------------------------------
//// 1. GETs
// curl -s localhost:8080   // !! default is GET
// hi from root/index path

// --------------------------------------------------------

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("\nServer running on localhost:8080 ...");

    let server = HttpServer::new(|| {
        App::new()
            .service(index)
    })
    .bind(("127.0.0.1", 8080))?
    .run();

    let handle = server.handle();

    // Timed Server:
    // Note: Comment out this to disable this feature and tokio
    // Spawn shutdown task
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        handle.stop(true).await; // true = graceful shutdown
    });

    server.await?;    
 
    println!("Server was shut-down");
    std::io::Result::Ok(())
}
