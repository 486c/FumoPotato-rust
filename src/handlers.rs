use futures::StreamExt;
use crate::fumo_context::FumoContext;

use std::sync::Arc;

use twilight_gateway::Event;
use twilight_gateway::cluster::Events;
use twilight_model::application::interaction::{ 
    Interaction, InteractionType, InteractionData,
    application_command::CommandData
};
use twilight_model::application::command::Command;
use twilight_util::builder::command::{ 
    CommandBuilder, StringBuilder
};
use twilight_model::application::command::CommandType;

use crate::commands::country_leaderboard;

use crate::utils::InteractionCommand;

use anyhow::Result;


async fn handle_commands(ctx: Arc<FumoContext>, cmd: InteractionCommand) {
    dbg!(&cmd);
    match cmd.data.name.as_str() {
        "leaderboard" | "Leaderboard" => country_leaderboard::run(&ctx, cmd).await,
        _ => {},
    }
}

pub async fn event_loop(ctx: Arc<FumoContext>, mut events: Events) {
    while let Some((shard_id, event)) = events.next().await {
        let ctx = Arc::clone(&ctx);

        tokio::spawn(async move { handle_event(ctx, shard_id, event).await });
        // TODO CHECK FOR ERROR
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

async fn handle_event(ctx: Arc<FumoContext>, shard_id: u64, event: Event) {
    match event {
        Event::InteractionCreate(c) => handle_interactions(ctx, c.0).await,
        _ => println!("Got unhandled event: {:?}", event),
    }
}

