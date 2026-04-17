use actix_web::middleware::Logger;
use env_logger::Env;

//// Topic
// Logging middleware — how to log every HTTP request/response automatically
// using Logger and env_logger


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    use actix_web::{App, HttpServer};

    env_logger::init_from_env(Env::default().default_filter_or("info"));

    HttpServer::new(|| {
        App::new()
            .wrap(Logger::default())
            .wrap(Logger::new("%a %{User-Agent}i"))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

// see "... actix_web::middleware::logger ..."
/* 
[2026-04-17T06:45:40Z INFO  actix_server::builder] starting 8 workers
[2026-04-17T06:45:40Z INFO  actix_server::server] Actix runtime found; starting in Actix runtime
[2026-04-17T06:45:40Z INFO  actix_server::server] starting service: "actix-web-service-127.0.0.1:8080", workers: 8, listening on: 127.0.0.1:8080

// Logs after browsing http://127.0.0.1:8080/users/11/alex

[2026-04-17T06:46:00Z INFO  actix_web::middleware::logger] 127.0.0.1 Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/147.0.0.0 Safari/537.36

[2026-04-17T06:46:00Z INFO  actix_web::middleware::logger] 127.0.0.1 "GET /users/11/alex HTTP/1.1" 404 0 "-" "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/147.0.0.0 Safari/537.36" 0.001770
 */