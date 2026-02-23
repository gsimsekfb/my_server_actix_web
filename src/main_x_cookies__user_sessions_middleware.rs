use actix_session::{Session, SessionMiddleware, storage::CookieSessionStore};
use actix_web::{web, App, Error, HttpResponse, HttpServer, cookie::Key};

// User sessions = remembering data about a user across multiple requests.
// The problem:
// HTTP is stateless - each request is independent. Server doesn't know 
// if two requests are from the same user.
// The solution:
// Sessions store user-specific data and track it via cookies.

async fn index(session: Session) -> Result<HttpResponse, Error> {
    // access session data
    if let Some(count) = session.get::<i32>("counter")? {
        session.insert("counter", count + 1)?;
        dbg!("a");
    } else {
        session.insert("counter", 1)?;
        dbg!("b");
    }

    Ok(HttpResponse::Ok().body(format!(
        "Count is {:?}!",
        session.get::<i32>("counter")?.unwrap()
    )))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .wrap(
                // create cookie based session middleware
                SessionMiddleware::builder(
                    CookieSessionStore::default(), // store in cookies
                    Key::from(&[0u8; 64])          // encryption key
                )
                .cookie_secure(false)
                .build()
            )
            .service(web::resource("/").to(index))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

// Note: In this setup, cookie changed each time since counter is stored in it
//
// 1.
// Client sends first request with no cookie.
// Server detects no existing session.
// Server creates new session and sets counter = 1.
// Server encrypts session data into a cookie.
// Client stores the cookie locally.
// curl -v -c cookies.txt "localhost:8080"
    // ...
    // < HTTP/1.1 200 OK
    // < content-length: 11
    // * Added cookie id="uPjFthBRmmleuvOsQEuvP505+Y%2F56gG5xl2oVNZ5I8bWqxSLEEWPAM08Tg%3D%3D" for domain localhost, path /, expire 0
    // < set-cookie: id=uPjFthBRmmleuvOsQEuvP505+Y%2F56gG5xl2oVNZ5I8bWqxSLEEWPAM08Tg%3D%3D; HttpOnly; SameSite=Lax; Path=/
    // < date: Fri, 13 Feb 2026 12:11:45 GMT
    // <
    // Count is 1!* 

// 2.
// Client sends stored cookie with second request.
// Server increments counter to 2 and re-encrypts.
// Server sends updated cookie back to client.
// curl -v -c cookies.txt -b cookies.txt "localhost:8080"
    // ...
    // > Cookie: id=uPjFthBRmmleuvOsQEuvP505+Y%2F56gG5xl2oVNZ5I8bWqxSLEEWPAM08Tg%3D%3D
    // >
    // < HTTP/1.1 200 OK
    // < content-length: 11
    // * Replaced cookie id="AlHi8WiTbOWX569%2FizRb6+mZC8MGOQ2xR38mO4TO17oDfJKhouY7v1aZxw%3D%3D" for domain localhost, path /, expire 0
    // < set-cookie: id=AlHi8WiTbOWX569%2FizRb6+mZC8MGOQ2xR38mO4TO17oDfJKhouY7v1aZxw%3D%3D; HttpOnly; SameSite=Lax; Path=/
    // < date: Fri, 13 Feb 2026 12:12:07 GMT
    // <
    // Count is 2!* 

// 3.
// curl -v -c cookies.txt -b cookies.txt "localhost:8080"
    // ...
    // > Cookie: id=AlHi8WiTbOWX569%2FizRb6+mZC8MGOQ2xR38mO4TO17oDfJKhouY7v1aZxw%3D%3D
    // >
    // < HTTP/1.1 200 OK
    // < content-length: 11
    // * Replaced cookie id="BEbGobk0aoESznNiTg8ti%2FZdupzVq2e46tkcsGDeEKLHFOXkntHSGzoYYw%3D%3D" for domain localhost, path /, expire 0
    // < set-cookie: id=BEbGobk0aoESznNiTg8ti%2FZdupzVq2e46tkcsGDeEKLHFOXkntHSGzoYYw%3D%3D; HttpOnly; SameSite=Lax; Path=/
    // < date: Fri, 13 Feb 2026 12:12:14 GMT
    // <
    // Count is 3!* 


