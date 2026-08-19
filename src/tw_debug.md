
## curl w/ verbose option

```
curl -v localhost:8080
* Host localhost:8080 was resolved.
* IPv6: ::1
* IPv4: 127.0.0.1
*   Trying [::1]:8080...
*   Trying 127.0.0.1:8080...
* Established connection to localhost (127.0.0.1 port 8080) from 127.0.0.1 port 2654
* using HTTP/1.x
> GET / HTTP/1.1
> Host: localhost:8080
> User-Agent: curl/8.21.0
> Accept: */*
>
* Request completely sent off
```

That's `curl` verbose output showing the request lifecycle:   
- DNS resolution found both IPv6 (`::1`) and IPv4 (`127.0.0.1`) for localhost
- it connected via IPv4
- sent the HTTP/1.1 GET request headers (`>` lines) 
- but no response has arrived yet, 
- meaning the server likely isn't responding (hanging or crashed).

## Debug Logs

### a. env_logger crate

Enable in main.rs:   
```rust
    env_logger::init();
```

```
set RUST_LOG=debug cargo r --bin twin
```
```
set RUST_LOG=debug && cargo r --bin twin    // windows
```
```
[2026-08-18T07:49:46Z INFO  actix_server::builder] starting 2 workers
[2026-08-18T07:49:46Z INFO  actix_server::server] Actix runtime found; starting in Actix runtime
[2026-08-18T07:49:46Z INFO  actix_server::server] starting service: "actix-web-service-127.0.0.1:8080", workers: 2, listening on: 127.0.0.1:8080
-- thread: ThreadId(2)
-- thread: ThreadId(3)
```

### b. Adding `actix_web::middleware::Logger`

```rust
    let http_server = HttpServer::new(move || { // move app_state into the closure
        App::new()
            .wrap(actix_web::middleware::Logger::default())
            // ...
```
```
-- thread: ThreadId(2)
[2026-08-18T07:50:24Z INFO  actix_web::middleware::logger] 127.0.0.1 "GET / HTTP/1.1" 200 472 "-" "curl/8.21.0" 0.001264
-- thread: ThreadId(3)
[2026-08-18T07:50:29Z INFO  actix_web::middleware::logger] 127.0.0.1 "GET / HTTP/1.1" 200 472 "-" "curl/8.21.0" 0.000785
```

## A Real Problem, Debugging and Solving 

### Problem

The handler parameter `feed` is as `web::Data<Arc<PriceFeed>` wrong.  
It should be `web::Data<AppState>`.  

```rust
#[get("/btc-price")]
async fn btc_price(feed: web::Data<Arc<PriceFeed>>) -> impl Responder {
    // ...
}
```

Related curl cmd:  
```rust
curl -is http://localhost:8080/btc-price
    HTTP/1.1 500 Internal Server Error
    content-length: 96
    content-type: text/plain; charset=utf-8
    date: Fri, 10 Jul 2026 09:29:06 GMT

    Requested application data is not configured correctly. 
    View/enable debug logs for more details
```


### Debug:

!! Make sure to use `RUST_LOG=debug cargo r ...` to see the needed logs.   

Related logs:  
```rust
RUST_LOG=debug cargo r --bin twin

-- Server starting on localhost:8080 ...

[2026-07-10T09:29:06Z DEBUG actix_web::data] Failed to extract `Data<alloc::sync::Arc<actix_hello::tw_error::PriceFeed>>` for `btc_price` handler. For the Data extractor to work correctly, wrap the data with `Data::new()` and pass it to `App::app_data()`. Ensure that types align in both the set and retrieve calls.
```

Enable logging in the code:  
```rust
use actix_web::{
    App, HttpServer, middleware::{Logger, from_fn}, web
};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("-- Server starting on localhost:8080 ...");
    println!("-- main's thread: {:?}", std::thread::current().id());

    // 1. simple logger
    env_logger::init();
    
    //... 

    // closure will be run per worker thread (at startup)
    // default workers: 8
    HttpServer::new(move || { // move app_state into the closure
        App::new()
            .wrap(Logger::default())
            // clone for each worker thread
            .app_data(app_state.clone()) // register the created data
            .route("/", web::get().to(index))
            // ...
            .service(buy_v2)
            .service(btc_price)
    })
    .workers(2)
    .bind(("127.0.0.1", 8080))?
    .run()
    .await?;
 
    println!("Server was shut-down");
    std::io::Result::Ok(())
}
```

