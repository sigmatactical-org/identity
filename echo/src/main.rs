//! Minimal HTTP echo server for identity E2E proxy tests.

#![forbid(unsafe_code)]

use std::convert::Infallible;

use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

async fn handle(req: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    if req.method() == Method::OPTIONS {
        return Ok(Response::builder()
            .status(StatusCode::NO_CONTENT)
            .body(Full::new(Bytes::new()))
            .expect("response"));
    }

    let (parts, body) = req.into_parts();
    let body_bytes = body
        .collect()
        .await
        .map(|collected| collected.to_bytes())
        .unwrap_or_default();

    let mut lines = vec![format!("{} request at {}", parts.method, parts.uri.path())];
    for (key, value) in parts.headers.iter() {
        lines.push(format!(
            "{}: {}",
            key.as_str().to_lowercase(),
            value.to_str().unwrap_or("")
        ));
    }
    if !body_bytes.is_empty() {
        lines.push(String::from_utf8_lossy(&body_bytes).into_owned());
    }

    let payload = lines.join("\n");
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/plain; charset=utf-8")
        .body(Full::new(Bytes::from(payload)))
        .expect("response"))
}

#[tokio::main]
async fn main() {
    let addr = std::env::var("ECHO_BIND").unwrap_or_else(|_| "0.0.0.0:3000".into());
    let listener = TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|err| panic!("bind {addr}: {err}"));
    eprintln!("sigma-identity-echo listening on {addr}");

    loop {
        let (stream, _) = match listener.accept().await {
            Ok(conn) => conn,
            Err(err) => {
                eprintln!("accept: {err}");
                continue;
            }
        };
        tokio::spawn(async move {
            let io = TokioIo::new(stream);
            if let Err(err) = http1::Builder::new()
                .serve_connection(io, service_fn(handle))
                .await
            {
                eprintln!("connection: {err:?}");
            }
        });
    }
}
