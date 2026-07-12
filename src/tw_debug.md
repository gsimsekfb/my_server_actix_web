

## Problem

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

