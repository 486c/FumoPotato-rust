use tokio_stream::StreamExt;
use crate::fumo_context::FumoContext;

use std::sync::Arc;

use twilight_gateway::Event;
use twilight_gateway::cluster::Events;
use twilight_model::application::interaction::{ 
    Interaction, InteractionData,
};
use twilight_model::application::command::Command;
use twilight_util::builder::command::{ 
    CommandBuilder, StringBuilder
};
use twilight_model::application::command::CommandType;

use crate::commands::country_leaderboard;

use crate::utils::InteractionCommand;

use eyre::Result;
use log::warn;

async fn handle_commands(ctx: Arc<FumoContext>, cmd: InteractionCommand) {
    dbg!(&cmd);
    let future = match cmd.data.name.as_str() {
        "leaderboard" | "Leaderboard" => country_leaderboard::run(&ctx, cmd),
        _ => return warn!("Got unhandled interaction command"),
    };

    match future.await {
        Ok(_) => {},
        Err(e) => println!("{:?}", e.wrap_err("Command failed"))
    }
}

pub async fn event_loop(ctx: Arc<FumoContext>, mut events: Events) {
    while let Some((shard_id, event)) = events.next().await {
        let ctx = Arc::clone(&ctx);

        tokio::spawn(async move { 
            let future = handle_event(ctx, shard_id, event);

            if let Err(e) = future.await {
                println!("{:?}", e.wrap_err("Failed to handle event"))
            }
        });
    }
}

pub fn global_commands() -> Vec<Command> {
    // TODO Move this somewhere else
    let mut commands: Vec<Command> = Vec::new();

    let cmd = CommandBuilder::new(
        "leaderboard",
        "Show country leaderboard",
        CommandType::ChatInput,
    )
    .option(
        StringBuilder::new("link", "direct link to beatmap")
        .required(false)
    ).build();
    commands.push(cmd);

    let cmd = CommandBuilder::new(
        "Leaderboard",
        "",
        CommandType::Message,
    ).build();
    commands.push(cmd);

    commands
}

async fn handle_interactions(ctx: Arc<FumoContext>, interaction: Interaction) {
    let Interaction {
        channel_id,
        data,
        guild_id,
        kind,
        id,
        token,
        ..
    } = interaction;

    match data {
        Some(InteractionData::ApplicationCommand(data)) => {
            let cmd = InteractionCommand {
                channel_id: channel_id.unwrap(),
                data,
                kind,
                guild_id,
                id,
                token
            };

            handle_commands(ctx, cmd).await;
        },
        _ => {},
    }
}

async fn handle_event(ctx: Arc<FumoContext>, shard_id: u64, event: Event) -> Result<()> {
    ctx.standby.process(&event);
    match event {
        Event::InteractionCreate(c) => handle_interactions(ctx, c.0).await,
        _ => {} //println!("Got unhandled event: {:?}", event),
    }

    Ok(())
}

