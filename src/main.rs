pub mod osu_api;
pub mod twitch_api;
pub mod fumo_context;
mod handlers;
mod commands;
mod utils;
mod database;
mod stats;
mod server;

use dotenv::dotenv;

use eyre::Result;

use std::env;
use std::sync::Arc;

use crate::fumo_context::FumoContext;
use crate::handlers::{ event_loop, global_commands };
use crate::server::run_server;

use tokio::signal;
use tokio::sync::oneshot::channel;

#[tokio::main(worker_threads = 8)]
async fn main() -> Result<()> {
    dotenv()?;

    let token = env::var("DISCORD_TOKEN")?;

    let (ctx, events) = FumoContext::new(&token).await;
    let ctx = Arc::new(ctx);

    ctx.cluster.up().await;

    let event_ctx = Arc::clone(&ctx);

    // Set global commands
    let application_id = ctx.http.current_user()
        .await?
        .model()
        .await?
        .id.cast();

    ctx.http.interaction(application_id)
        .set_global_commands(&global_commands())
        .await?;

    // Spawn twitch checker
    let (tx, recv) = channel::<()>();
    let twitch_ctx = Arc::clone(&ctx);
    tokio::spawn(async move {
        tokio::select! {
            _ = commands::twitch::twitch_worker(
                Arc::clone(&twitch_ctx)
            ) => {
                println!("Twitch checker loop sudenly ended!")
            }
            _ = recv => ()
        }
    });

    // Spawn http server
    let server_tx = {
        let server_ctx = Arc::clone(&ctx);
        let (tx, rx) = tokio::sync::oneshot::channel();
        tokio::spawn(run_server(server_ctx, rx));

        tx
    };

    // Run
    tokio::select! {
        _ = event_loop(event_ctx, events) => println!("Error in event loop!"),
        res = signal::ctrl_c() => match res {
            Ok(_) => println!("\nGot Ctrl+C"),
            Err(_) => println!("Can't get Cntrl+C signal for some reason"),
        }
    }
    
    // Close everything
    ctx.cluster.down();

    if tx.send(()).is_err() {
        println!("Failed to close twitch loop!");
    }
    println!("Closed twitch loop!");

    if server_tx.send(()).is_err() {
        println!("Failed to close http server!");
    }
    println!("Closed http server!");

    println!("Bye!!!");
    Ok(())
}
