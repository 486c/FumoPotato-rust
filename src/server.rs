use crate::fumo_context::FumoContext;

use std::{net::SocketAddr, sync::Arc};

use tokio::{net::TcpListener, sync::oneshot::Receiver};

use bytes::Bytes;
use http_body_util::Full;
use hyper::{server::conn::http1, service::service_fn, Request, Response};
use hyper_util::rt::tokio::TokioIo;
use prometheus::{Encoder, TextEncoder};

async fn metrics_handler(
    ctx: Arc<FumoContext>,
    _req: Request<hyper::body::Incoming>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let mut buf = Vec::new();
    let encoder = TextEncoder::new();
    let metric_families = ctx.stats.registry.gather();
    encoder.encode(&metric_families, &mut buf).unwrap();

    Ok(
        Response::builder()
            .header("Content-Type", "text/plain")
            .body(Bytes::from(buf).into())
            .expect("failed to build response for metrics endpoint")
    )
}

async fn service(
    ctx: Arc<FumoContext>,
    req: Request<hyper::body::Incoming>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    match req.uri().path() {
        "/metrics" => metrics_handler(ctx, req).await,
        _ => Ok(Response::default()),
    }
}

pub async fn server_loop(ctx: Arc<FumoContext>) {
    let addr = SocketAddr::from(([0, 0, 0, 0], 5000));

    let listener = TcpListener::bind(&addr).await.unwrap();

    loop {
        let (stream, _) = listener.accept().await.unwrap();
        let io = TokioIo::new(stream);

        let context = ctx.clone();
        tokio::task::spawn(async move {
            let _ = http1::Builder::new()
                .serve_connection(
                    io,
                    service_fn(|req| service(context.clone(), req)),
                )
                .await;
        });
    }
}

pub async fn run_server(ctx: Arc<FumoContext>, shutdown_rx: Receiver<()>) {
    tokio::select! {
        _ = server_loop(ctx.clone()) => println!("wtf"),
        _ = shutdown_rx => println!("Bye http server"),
    };
}
