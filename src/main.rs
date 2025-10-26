mod commands;
mod components;
pub mod fumo_context;
mod handlers;
mod server;
mod stats;
pub mod twitch_api;
mod utils;

use dotenv::dotenv;

use eyre::Result;
use twilight_gateway::CloseFrame;

use std::{env, sync::Arc, time::Duration};

use crate::{
    fumo_context::FumoContext,
    handlers::{event_loop, global_commands},
    server::run_server,
};
use twilight_interactions::command::CreateCommand;

use tokio::{signal, sync::oneshot::channel};

async fn spawn_twitch_worker(
    twitch_ctx: Arc<FumoContext>,
    rx: tokio::sync::oneshot::Receiver<()>,
) {
    tokio::spawn(async move {
        tokio::select! {
            _ = commands::twitch::twitch_worker(
                twitch_ctx.clone()
            ) => {
                tracing::error!("Twitch checker loop sudenly ended!")
            }
            _ = rx => {
            }
        }
    });
}

async fn spawn_osu_worker(
    ctx: Arc<FumoContext>,
    rx: tokio::sync::oneshot::Receiver<()>,
) {
    tokio::spawn(async move {
        tokio::select! {
            _ = commands::osu_tracking::osu_tracking_worker(
                ctx.clone()
            ) => {
                tracing::error!("Osu tracking checker loop sudenly ended!");
            }
            _ = rx => {
            }
        }
    });
}

#[tokio::main(worker_threads = 4)]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .with_thread_names(false)
        .init();

    dotenv()?;

    let token = env::var("DISCORD_TOKEN")?;

    let (ctx, mut shards) = FumoContext::new(&token).await?;
    let ctx = Arc::new(ctx);

    let application_id =
        ctx.http.current_user().await?.model().await?.id.cast();

    tracing::info!("Setting global commands...");

    // Mixing manually created commands
    // and twilight-interactions created commands :)
    let mut commands = global_commands();

    commands.push(commands::osu::OsuCommands::create_command().into());

    commands.push(
        commands::multiplayer::MultiplayerCommands::create_command().into(),
    );

    commands.push(
        commands::country_leaderboard::LeaderboardCommand::create_command()
            .into(),
    );

    // Set global commands
    ctx.http
        .interaction(application_id)
        .set_global_commands(&commands)
        .await?;

    // Spawn twitch checker
    let (twitch_tx, rx) = channel::<()>();
    let twitch_ctx = Arc::clone(&ctx);
    spawn_twitch_worker(twitch_ctx, rx).await;

    // Spawn osu checker
    let (osu_tx, rx) = channel::<()>();
    let osu_tracker_ctx = Arc::clone(&ctx);
    spawn_osu_worker(osu_tracker_ctx, rx).await;

    // Spawn http server
    let server_tx = {
        let server_ctx = Arc::clone(&ctx);
        let (tx, rx) = tokio::sync::oneshot::channel();
        tokio::spawn(run_server(server_ctx, rx));

        tx
    };

    // Run discord event loop
    let event_ctx = Arc::clone(&ctx);

    tokio::select! {
        _ = event_loop(event_ctx, &mut shards) => tracing::error!("Error in event loop!"),
        res = signal::ctrl_c() => match res {
            Ok(_) => tracing::info!("Got Ctrl+C"),
            Err(_) => tracing::error!("Can't get Cntrl+C signal for some reason"),
        }
    }

    // Close everything
    for shard in shards.iter_mut() {
        let reason = CloseFrame::new(1000, "Closing connection");
        let res = shard.close(reason).await;

        match res {
            Ok(_) => tracing::info!("Closed shard"),
            Err(e) => tracing::error!("Failed to close shard: {}", e),
        }
    }

    if twitch_tx.send(()).is_err() {
        tracing::error!("Failed to close twitch loop!");
    }

    tracing::info!("Closed twitch loop!");

    if server_tx.send(()).is_err() {
        tracing::error!("Failed to close http server!");
    }

    tracing::info!("Closed http server!");

    if osu_tx.send(()).is_err() {
        tracing::error!("Failed to close osu tracking loop!");
    }

    commands::twitch::twitch_sync_db(ctx.clone())
        .await
        .expect("Failed to sync checker list with db");

    // Wait for all threads complete peacefully
    tokio::time::sleep(Duration::from_secs(5)).await;

    ctx.state.lock().await.sync_to_disk();

    tracing::info!("Bye!!!");

    Ok(())
}
