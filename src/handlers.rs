use crate::{
    commands::{
        country_leaderboard::LeaderboardCommand,
        multiplayer::MultiplayerCommands, osu::OsuCommands,
    },
    fumo_context::FumoContext,
};
use tokio_stream::StreamExt;
use twilight_gateway::stream::ShardEventStream;

use std::sync::Arc;

use twilight_gateway::{Event, Shard};
use twilight_model::application::{
    command::{Command, CommandType},
    interaction::{Interaction, InteractionData},
};
use twilight_util::builder::command::{
    CommandBuilder, StringBuilder, SubCommandBuilder,
};

use crate::commands::{country_leaderboard, twitch};

use crate::utils::interaction::InteractionCommand;

use eyre::Result;

async fn handle_commands(ctx: Arc<FumoContext>, cmd: InteractionCommand) {
    let res = match cmd.data.name.as_str() {
        "Leaderboard" => country_leaderboard::run(&ctx, cmd).await,
        "leaderboard" => LeaderboardCommand::handle(&ctx, cmd).await,
        "twitch" => twitch::run(&ctx, cmd).await,
        "osu" => OsuCommands::handle(&ctx, cmd).await,
        "multiplayer" => MultiplayerCommands::handle(&ctx, cmd).await,
        _ => {
            return tracing::error!(
                "Got unhandled interaction command: {}",
                cmd.data.name.as_str()
            )
        }
    };

    // TODO Add some basic error message i guess
    match res {
        Ok(_) => {}
        Err(e) => tracing::error!("Failed to handle command: {e}"),
    }
}

pub async fn event_loop(
    ctx: Arc<FumoContext>,
    shards: &mut [Shard],
) -> eyre::Result<()> {
    let mut events = ShardEventStream::new(shards.iter_mut());

    while let Some((shard, event)) = events.next().await {
        let event = match event {
            Ok(event) => event,
            Err(source) => {
                tracing::error!("Received a failed event: {}", source);

                if source.is_fatal() {
                    tracing::error!("Got fatal shard event: {}", source);
                    return Err(source.into());
                };

                continue;
            }
        };

        let ctx = Arc::clone(&ctx);
        let shard_id = shard.id().number();

        tokio::spawn(async move {
            let future = handle_event(ctx, shard_id, event);

            if let Err(e) = future.await {
                tracing::error!("Failed to handle event: {:?}", e);
            };
        });
    }

    tracing::warn!("Closing event loop");
    Ok(())
}

pub fn global_commands() -> Vec<Command> {
    // TODO Make this more readable i guess
    // mb use twilight-interactions?
    let mut commands: Vec<Command> = Vec::new();

    let cmd =
        CommandBuilder::new("Leaderboard", "", CommandType::Message).build();
    commands.push(cmd);

    // twitch
    let cmd = CommandBuilder::new(
        "twitch",
        "twitch related commands",
        CommandType::ChatInput,
    )
    .option(
        SubCommandBuilder::new("add", "add twitch channel to tracking").option(
            StringBuilder::new("name", "twitch channel name").required(true),
        ),
    )
    .option(
        SubCommandBuilder::new("remove", "remove twitch channel from tracking")
            .option(
                StringBuilder::new("name", "twitch channel name")
                    .required(true),
            ),
    )
    .option(SubCommandBuilder::new(
        "list",
        "list tracked twitch channels that being tracked on current channel",
    ))
    .build();
    commands.push(cmd);

    commands
}

async fn handle_interactions(
    ctx: Arc<FumoContext>,
    interaction: Interaction,
) -> Result<()> {
    let Interaction {
        channel,
        data,
        // guild_id,
        // kind,
        id,
        token,
        member,
        user,
        ..
    } = interaction;

    match data {
        Some(InteractionData::ApplicationCommand(data)) => {
            let cmd = InteractionCommand {
                channel_id: channel.unwrap().id,
                data: *data,
                // kind,
                // guild_id,
                id,
                token,
                member,
                user,
            };

            handle_commands(ctx, cmd).await;
        }
        Some(InteractionData::MessageComponent(_)) => {}
        Some(InteractionData::ModalSubmit(_)) => {}
        _ => {}
    };

    Ok(())
}

async fn handle_event(
    ctx: Arc<FumoContext>,
    _shard_id: u64,
    event: Event,
) -> Result<()> {
    ctx.stats
        .bot
        .discord_events
        .with_label_values(&["incoming"])
        .inc();

    ctx.standby.process(&event);

    if let Event::InteractionCreate(c) = event {
        handle_interactions(ctx, c.0).await?
    }

    Ok(())
}
