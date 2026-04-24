#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(unused_variables)]
#![allow(unused)]

use std::task::Poll;

use actix_web::{
    App, Error, HttpRequest, HttpResponse, HttpServer, 
    http::{self, StatusCode, header::ContentEncoding}, web
};
use futures::stream;


// Topics:
// Stream Response Testing: 
//
// 1. Test full payload
// 2. Test chunk by chunk
//
// For example testing SSE = Server-Sent Events which are:
// - Server pushes data to client over a single HTTP
// - Server → Client only
// - Live feeds, notifications


// SSE = Server-Sent Events example
// e.g.
// Stream generation, create these byte chunks:
//      b"data: 5\n\n
//      b"data: 4\n\n
//      b"data: 3\n\n
//      b"data: 2\n\n
//      b"data: 1\n\n
//
async fn sse(_req: HttpRequest) -> HttpResponse {
    let mut counter: usize = 5;

    // yields `data: N` where N in [5; 1]
    let server_events =
        stream::poll_fn(move |_cx| -> Poll<Option<Result<web::Bytes, Error>>> {
            if counter == 0 { return Poll::Ready(None); }

            let payload = format!("data: {}\n\n", counter);
            counter -= 1;
            Poll::Ready(Some(Ok(web::Bytes::from(payload))))
        });

    HttpResponse::build(StatusCode::OK)
        .insert_header((http::header::CONTENT_TYPE, "text/event-stream"))
        .insert_header(ContentEncoding::Identity)
        .streaming(server_events)
}

#[cfg(test)]
mod tests {
    use super::*;

    use actix_web::{body, body::MessageBody as _, rt::pin, test, web, App};
    use futures::future;

    // 1. Test full payload:
    //      b"data: 5\n\ndata: 4\n\ndata: 3\n\ndata: 2\n\ndata: 1\n\n"
    #[actix_web::test]
    async fn test_stream_full_payload() {
        let app = test::init_service(App::new().route("/", web::get().to(sse))).await;
        let req = test::TestRequest::get().to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let body = resp.into_body();
        let bytes = body::to_bytes(body).await;
        assert_eq!(
            bytes.unwrap(),
            web::Bytes::from_static(
                b"data: 5\n\ndata: 4\n\ndata: 3\n\ndata: 2\n\ndata: 1\n\n"
            )
        );
    }

    // 2. Test chunk by chunk:
    //      b"data: 5\n\n
    //      b"data: 4\n\n
    //      b"data: 3\n\n
    //      b"data: 2\n\n
    //      b"data: 1\n\n
    #[actix_web::test]
    async fn test_stream_chunk() {
        let app = test::init_service(App::new().route("/", web::get().to(sse))).await;
        let req = test::TestRequest::get().to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let body = resp.into_body(); // future
        pin!(body);

        // first chunk
        let bytes = future::poll_fn(|cx| body.as_mut().poll_next(cx)).await;
        assert_eq!(
            bytes.unwrap().unwrap(),
            web::Bytes::from_static(b"data: 5\n\n")
        );

        // second chunk
        let bytes = future::poll_fn(|cx| body.as_mut().poll_next(cx)).await;
        assert_eq!(
            bytes.unwrap().unwrap(),
            web::Bytes::from_static(b"data: 4\n\n")
        );

        // remaining chunks
        for i in 0..3 {
            let expected_data = format!("data: {}\n\n", 3 - i);
            let bytes = future::poll_fn(|cx| body.as_mut().poll_next(cx)).await;
            assert_eq!(bytes.unwrap().unwrap(), web::Bytes::from(expected_data));
        }
    }
}

//// Not needed, could be used for debugging
//
// #[actix_web::main]
// async fn main() {
//     HttpServer::new(|| {
//         App::new().route("/", web::get().to(sse));
//     })
//     .bind(("127.0.0.1", 8080))?
//     .run()
//     .await;
// }
