use tokio_stream::StreamExt;
use twilight_gateway::stream::ShardEventStream;
use crate::commands::multiplayer::MultiplayerCommands;
use crate::commands::osu::OsuCommands;
use crate::fumo_context::FumoContext;

use std::sync::Arc;

use twilight_gateway::{Event, Shard};
use twilight_model::application::interaction::{ 
    Interaction, InteractionData,
};
use twilight_model::application::command::Command;
use twilight_util::builder::command::{ 
    CommandBuilder, StringBuilder, SubCommandBuilder,
};
use twilight_model::application::command::CommandType;

use crate::commands::{
    country_leaderboard,
    twitch,
};

use crate::utils::InteractionCommand;

use eyre::Result;

async fn handle_commands(
    ctx: Arc<FumoContext>, 
    cmd: InteractionCommand,
) {
    let res = match cmd.data.name.as_str() {
        "leaderboard" | "Leaderboard" => 
            country_leaderboard::run(&ctx, cmd).await,
        "twitch" => twitch::run(&ctx, cmd).await,
        "osu" => OsuCommands::handle(&ctx, cmd).await,
        "multiplayer" => MultiplayerCommands::handle(&ctx, cmd).await,
        _ => return println!("Got unhandled interaction command"),
    };
    
    // TODO Add some basic error message i guess
    match res {
        Ok(_) => {},
        Err(e) => println!("{:?}", e.wrap_err("Command failed"))
    }
}

pub async fn event_loop(ctx: Arc<FumoContext>, shards: &mut [Shard]) {
    let mut events = ShardEventStream::new(shards.iter_mut());

    loop {
        match events.next().await {
            Some((shard, Ok(event))) => {
               let ctx = Arc::clone(&ctx);
               let shard_id = shard.id().number();

               tokio::spawn(async move { 
                   let future = handle_event(
                       ctx, 
                       shard_id,
                       event
                    );

                   if let Err(e) = future.await {
                       println!(
                           "{:?}",
                           e.wrap_err("Failed to handle event")
                        )
                   }
               });
            },
            Some((_shard, Err(error))) => {
                if error.is_fatal() {
                    println!("Got fatal shard event! Quiting event loop!");
                    break;
                }

                continue;
            },
            None => return,
        };
    }
}

pub fn global_commands() -> Vec<Command> {
    // TODO Make this more readable i guess
    // mb use twilight-interactions?
    let mut commands: Vec<Command> = Vec::new();

    /* osu */
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

    /* twitch */
    let cmd = CommandBuilder::new(
        "twitch",
        "twitch related commands",
        CommandType::ChatInput,
    )
    .option(
        SubCommandBuilder::new("add", "add twitch channel to tracking")
        .option(
            StringBuilder::new("name", "twitch channel name")
            .required(true)
        )
    )
    .option(
        SubCommandBuilder::new("remove", "remove twitch channel from tracking")
        .option(
            StringBuilder::new("name", "twitch channel name")
            .required(true)
        )
    )
    .option(
        SubCommandBuilder::new("list", "list tracked twitch channels that being tracked on current channel")
    )
    .build();
    commands.push(cmd);


    commands
}

async fn handle_interactions(
    ctx: Arc<FumoContext>, 
    interaction: Interaction
) -> Result<()> {
    let Interaction {
        channel,
        data,
        guild_id,
        kind,
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
                kind,
                guild_id,
                id,
                token,
                member,
                user
            };

            handle_commands(ctx, cmd).await;
        },
        Some(InteractionData::MessageComponent(_)) => {},
        Some(InteractionData::ModalSubmit(_)) => {},
        _ => {},
    };

    Ok(())
}

async fn handle_event(
    ctx: Arc<FumoContext>, 
    _shard_id: u64, 
    event: Event
) -> Result<()> {
    ctx.standby.process(&event);
    match event {
        Event::InteractionCreate(c) => 
            handle_interactions(ctx, c.0).await?,
        _ => {}
    }

    Ok(())
}

