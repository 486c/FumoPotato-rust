use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::id::{ Id, marker::ChannelMarker };
use twilight_util::builder::embed::{ 
    EmbedBuilder, EmbedAuthorBuilder, EmbedFooterBuilder,
    image_source::ImageSource,
};

use crate::twitch_api::{ TwitchStream, StreamType };
use crate::fumo_context::FumoContext;
use crate::utils::{ MessageBuilder, InteractionCommand };

use eyre::{ Result, bail };

use std::{ slice, sync::Arc, time::Duration };

pub async fn twitch_worker(ctx: Arc<FumoContext>) {
    println!("Started twitch checker loop!");
    loop {
        match twitch_checker(&ctx).await {
            Ok(_) => {},
            Err(e) => {
                println!("{:?}", 
                    e.wrap_err("Error occured inside twitch tracking loop!")
                );
            }
        }
        tokio::time::sleep(Duration::from_secs(120)).await;
    }
}

pub async fn announce_channel(
    ctx: &FumoContext, 
    channel_id: Id<ChannelMarker>,
    c: &TwitchStream
) -> Result<()> {
    let author = EmbedAuthorBuilder::new(format!("{} is live!", &c.user_name))
        .url(format!("https://twitch.tv/{}", &c.user_name))
        .build();

    let source = ImageSource::url(format!(
        "https://static-cdn.jtvnw.net/previews-ttv/live_user_{}-1280x720.jpg",
            &c.user_name
    ))?;

    let source_footer = ImageSource::url(format!(
        "https://static-cdn.jtvnw.net/ttv-boxart/{}-250x250.jpg",
            c.game_id
    ))?;

    let footer = EmbedFooterBuilder::new(&c.game_name)
        .icon_url(source_footer)
        .build();

    let embed = EmbedBuilder::new()
        .color(0x97158a)
        .description(&c.title)
        .image(source)
        .footer(footer)
        .author(author)
        .build();

    ctx.http.create_message(channel_id)
        .embeds(slice::from_ref(&embed))?
        .await?;

    Ok(())
}

pub async fn twitch_checker(ctx: &FumoContext) -> Result<()> {
    let streamers  = ctx.db.get_streamers().await?;

    for streamer_db in streamers.iter() {
        let name = &streamer_db.name;
        let online = streamer_db.online;

        match ctx.twitch_api.get_stream(name).await {
            Some(streamer) => {
                if streamer.stream_type == StreamType::Live && !online {
                    ctx.db.toggle_online(name).await?;

                    let channels = ctx.db.get_channels(name).await?;

                    for channel in channels.iter() {
                        let channel_id: Id<ChannelMarker> = Id::new(channel.id as u64);
                        announce_channel(ctx, channel_id, &streamer).await?;
                    }
                }
    
                if streamer.stream_type == StreamType::Offline && online {
                    ctx.db.toggle_online(name).await?;
                }
            }
            None => {
                println!("Got unexpected None during twitch_check iteration");
                println!("{}", &name);
            }
        }
    }       

    Ok(())
}

/*

async fn twitch_list(
    ctx: &FumoContext, 
    command: &InteractionCommand, 
    name: &str)
-> Result<()> {
    todo!()
}

async fn twitch_check(
    ctx: &FumoContext, 
    command: &InteractionCommand, 
    name: &str)
-> Result<()> {
    todo!()
}
*/

async fn twitch_add(
    ctx: &FumoContext, 
    command: &InteractionCommand, 
    name: &str)
-> Result<()> {
    command.defer(ctx).await?;
    let mut msg = MessageBuilder::new();

    // Checking if user with provided name actually exists
    if (ctx.twitch_api.get_user_by_name(name).await).is_none() {
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
            Ok(())
        },
        None => {
            ctx.db.add_tracking(&streamer, channel_id).await?;
            msg = msg.content(
                format!("Successfully added `{name}` to tracking!")
            );
            command.update(ctx, &msg).await?;
            Ok(())
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
            Ok(())
        },
        None => {
            msg = msg.content(
                format!("`{name}` doesn't exists on current channel!")
            );
            command.update(ctx, &msg).await?;
            Ok(())
        }
    }
}

pub async fn run(ctx: &FumoContext, command: InteractionCommand) -> Result<()> {
    if let Some(option) = &command.data.options.get(0) {
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
