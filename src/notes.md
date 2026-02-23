

1. Where is returning `HttpResponse` or `impl Responder` ? 
```rust
#[get("/")]
async fn index(data: web::Data<AppState>) -> String {
    let app_name = &data.app_name;
    format!("Hello {app_name}!")
}
```
Actix has a blanket implementation: any type that implements Responder can be returned directly.





