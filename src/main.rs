pub mod osu_api;
pub mod twitch_api;
pub mod fumo_context;
mod handlers;
mod commands;
mod utils;
mod database;

use dotenv::dotenv;

use eyre::Result;

use std::env;
use std::sync::Arc;

use crate::fumo_context::FumoContext;
use crate::handlers::{ event_loop, global_commands };

use tokio::signal;

#[tokio::main(worker_threads = 8)]
async fn main() -> Result<()> {
    dotenv().unwrap();

    let token = env::var("DISCORD_TOKEN").unwrap();

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

    // Run
    tokio::select! {
        _ = event_loop(event_ctx, events) => println!("Error in event loop!"),
        res = signal::ctrl_c() => match res {
            Ok(_) => println!("\nGot Ctrl+C"),
            Err(_) => println!("Can't get Cntrl+C signal for some reason"),
        }
    }

    ctx.cluster.down();

    println!("Bye!!!");
    Ok(())
}
