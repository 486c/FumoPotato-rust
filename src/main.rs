pub mod osu_api;
pub mod twitch_api;
mod fumo_context;

use dotenv::dotenv;

use std::env;
use std::sync::Arc;

use futures::StreamExt;
use crate::fumo_context::FumoContext;

use twilight_http::Client;
use twilight_gateway::{ EventTypeFlags, Intents, Cluster, Event };
use twilight_gateway::cluster::Events;
use twilight_model::application::interaction::{ 
    Interaction, InteractionType, InteractionData,
    application_command::CommandData
};
use twilight_model::id::{
    Id, 
    marker::{ ChannelMarker, GuildMarker }
};

use tokio::signal;

#[derive(Debug)]
pub struct InteractionCommand {
    channel_id: Id<ChannelMarker>,
    data: Box<CommandData>,
    kind: InteractionType,
    guild_id: Option<Id<GuildMarker>>,
}

async fn test_command(ctx: &FumoContext, cmd: InteractionCommand) { 
    let _ = ctx.http.create_message(cmd.channel_id)
        .content("test!").unwrap()
        .exec()
        .await.unwrap();
}

async fn handle_commands(ctx: Arc<FumoContext>, cmd: InteractionCommand) {
    dbg!(&cmd);
    match cmd.data.name.as_str() {
        "leaderboard" => test_command(&ctx, cmd).await,
        _ => {},
    }
}

async fn handle_interactions(ctx: Arc<FumoContext>, interaction: Interaction) {
    let Interaction {
        channel_id,
        data,
        guild_id,
        kind,
        ..
    } = interaction;

    match data {
        Some(InteractionData::ApplicationCommand(data)) => {
            let cmd = InteractionCommand {
                channel_id: channel_id.unwrap(),
                data,
                kind,
                guild_id
            };

            handle_commands(ctx, cmd).await;
        },
        _ => {},
    }
}

async fn handle_event(ctx: Arc<FumoContext>, shard_id: u64, event: Event) {
    match event {
        Event::InteractionCreate(c) => handle_interactions(ctx, c.0).await,
        _ => println!("Got unhandled event: {:?}", event),
    }
}

async fn event_loop(ctx: Arc<FumoContext>, mut events: Events) {
    while let Some((shard_id, event)) = events.next().await {
        let ctx = Arc::clone(&ctx);

        tokio::spawn(async move { handle_event(ctx, shard_id, event).await });
        // TODO CHECK FOR ERROR
    }
}

#[tokio::main(worker_threads = 8)]
async fn main() {
    dotenv().unwrap();

    let token = env::var("DISCORD_TOKEN").unwrap();

    let (ctx, events) = FumoContext::new(&token).await;
    let ctx = Arc::new(ctx);

    ctx.cluster.up().await;

    let event_ctx = Arc::clone(&ctx);
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
}
