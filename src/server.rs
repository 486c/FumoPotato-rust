use crate::fumo_context::FumoContext;

use std::sync::Arc;

use tokio::sync::oneshot::Receiver;

use hyper::{Body, Request, Response, Server, StatusCode};
use routerify::{Middleware, Router, RouterService, RequestInfo};
use std::{convert::Infallible, net::SocketAddr};
use routerify::prelude::*;
use prometheus::{Encoder, TextEncoder};

async fn metrics_handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let mut buf = Vec::new();
    let encoder = TextEncoder::new();
    let ctx = req.data::<Arc<FumoContext>>().unwrap();
    let metric_families = ctx.stats.registry.gather();
    encoder.encode(&metric_families, &mut buf).unwrap();

    Ok(Response::new(Body::from(buf)))
}

fn router(ctx: Arc<FumoContext>) -> Router<Body, Infallible> {
    Router::builder()
        .data(ctx)
        .get("/metrics", metrics_handler)
        .build()
        .unwrap()
}

pub async fn run_server(ctx: Arc<FumoContext>, shutdown_rx: Receiver<()>) {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let router = router(ctx);

    let service = RouterService::new(router).unwrap();

    let server = Server::bind(&addr).serve(service)
        .with_graceful_shutdown(async {
            let _ = shutdown_rx.await;
        });
    
    println!("Started http server!");

    if let Err(err) = server.await {
        eprintln!("Server error: {}", err);
   }

}
