use crate::fumo_context::FumoContext;

use std::{sync::Arc, net::SocketAddr};

use tokio::{sync::oneshot::Receiver, net::TcpListener };

use hyper::{ Request, Response };
use prometheus::{ Encoder, TextEncoder };
use http_body_util::Full;
use hyper::service::service_fn;
use bytes::Bytes;
use hyper_util::rt::tokio::TokioIo;
use hyper::server::conn::http1;


async fn metrics_handler(
    ctx: Arc<FumoContext>,
    _req: Request<hyper::body::Incoming>
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let mut buf = Vec::new();
    let encoder = TextEncoder::new();
    let metric_families = ctx.stats.registry.gather();
    encoder.encode(&metric_families, &mut buf).unwrap();

    Ok(Response::new(Bytes::from(buf).into()))
}

async fn service(
    ctx: Arc<FumoContext>,
    req: Request<hyper::body::Incoming>
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    match req.uri().path() {
        "/metrics" => metrics_handler(ctx, req).await,
        _ => Ok(Response::default())
    }
}

pub async fn server_loop(ctx: Arc<FumoContext>) {
    let addr = SocketAddr::from(([127, 0, 0, 1], 5000));

    let listener = TcpListener::bind(&addr).await.unwrap();

    loop {
        let (stream, _) = listener.accept().await.unwrap();
        let io = TokioIo::new(stream);

        let context = ctx.clone();
        tokio::task::spawn(async move {
            let _ = http1::Builder::new()
                .serve_connection(io, service_fn(|req|
                    service(context.clone(), req)
                ))
                .await;
        });
    }
}

pub async fn run_server(ctx: Arc<FumoContext>, shutdown_rx: Receiver<()>) {
    tokio::select!{
        _ = server_loop(ctx.clone()) => println!("wtf"),
        _ = shutdown_rx => println!("Bye"),
    };

    println!("Bye http server");
}

    
    /*
    let service = RouterService::new(router).unwrap();

    let server = Server::bind(&addr).serve(service)
        .with_graceful_shutdown(async {
            let _ = shutdown_rx.await;
        });
    
    println!("Started http server!");

    if let Err(err) = server.await {
        eprintln!("Server error: {err}");
   }
   */

