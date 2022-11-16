use crate::fumo_context::FumoContext;
use crate::utils::{ MessageBuilder, InteractionCommand };
use twilight_model::application::interaction::application_command::CommandOptionValue;

use eyre::{ Result, bail };

async fn twitch_add(
    ctx: &FumoContext, 
    command: &InteractionCommand, 
    name: &str)
-> Result<()> {
    command.defer(ctx).await?;
    let mut msg = MessageBuilder::new();

    // Checking if user with provided name actually exists
    if let None = ctx.twitch_api.get_user_by_name(name).await {
        msg = msg.content(
            format!("User with name `{name}` does not exists on twitch!")
        );
        command.update(ctx, &msg).await?;
        return Ok(())
    };

    let streamer = match ctx.db.get_streamer(name).await {
        Some(s) => s,
        None => ctx.db.add_streamer(name).await?,
    };
    
    let channel_id: i64 = command.channel_id.get().try_into()?;
    match ctx.db.get_tracking(name, channel_id).await {
        Some(_) => {

            msg = msg.content(
                format!("`{name}` already added to current channel!")
            );
            command.update(ctx, &msg).await?;
            return Ok(());
        },
        None => {
            ctx.db.add_tracking(&streamer, channel_id).await?;
            msg = msg.content(
                format!("Successfully added `{name}` to tracking!")
            );
            command.update(ctx, &msg).await?;
            return Ok(());
        },
    }
}

async fn twitch_remove(
    ctx: &FumoContext, 
    command: &InteractionCommand, 
    name: &str
) -> Result<()> {
    command.defer(ctx).await?;
    let mut msg = MessageBuilder::new();

    let channel_id: i64 = command.channel_id.get().try_into()?;

    match ctx.db.get_tracking(name, channel_id).await {
        Some(_) => {
            ctx.db.remove_tracking(name, channel_id).await?;
            msg = msg.content(
                format!("Successfully removed `{name}` from current channel!")
            );
            command.update(ctx, &msg).await?;
            return Ok(());
        },
        None => {
            msg = msg.content(
                format!("`{name}` doesn't exists on current channel!")
            );
            command.update(ctx, &msg).await?;
            return Ok(());
        }
    }
}

pub async fn run(ctx: &FumoContext, command: InteractionCommand) -> Result<()> {
    if let Some(ref option) = &command.data.options.get(0) {
        match &option.value {
            CommandOptionValue::SubCommand(args) => {
                if let CommandOptionValue::String(name) = &args[0].value {
                    match option.name.as_ref() {
                        "add" => Ok(twitch_add(ctx, &command, name).await?),
                        "remove" => Ok(twitch_remove(ctx, &command, name).await?),
                        &_ => bail!("Unrecognized option name `{}`", option.name),
                    }
                } else {
                    bail!("Failed to extract required argument from subcommand")
                }
            },
            _ => {
                bail!("Unrecognized option type in subcommand `{}`", option.name)
            }
        }
    } else {
        bail!("Required option is not found")
    }
}
