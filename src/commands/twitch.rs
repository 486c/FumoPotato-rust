use std::{ slice, sync::Arc, time::Duration, fmt::Write };

use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::{
    id::{ Id, marker::ChannelMarker },
    http::attachment::Attachment,
};
use twilight_util::builder::embed::{ 
    EmbedBuilder, EmbedAuthorBuilder, EmbedFooterBuilder,
    image_source::ImageSource,
};

use rand::distributions::{Alphanumeric, DistString};

use crate::twitch_api::{ TwitchStream, StreamType };
use crate::fumo_context::FumoContext;
use crate::utils::{ MessageBuilder, InteractionCommand};
use crate::database::twitch::TwitchTrackedStreamer;

use crate::random_string;

use eyre::{ Result, bail };

pub async fn twitch_worker(ctx: Arc<FumoContext>) {
    println!("Started twitch checker loop!");
    loop {
        if let Err(e) = twitch_checker(&ctx).await {
            println!("{:?}", 
                e.wrap_err("Error occured inside twitch tracking loop!")
            );
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
        .url(format!("https://twitch.tv/{}", &c.user_login))
        .build();

    let image_link =  format!(
        "https://static-cdn.jtvnw.net/previews-ttv/live_user_{}-1280x720.jpg",
        &c.user_login
    );

    let image = ctx.twitch_api
        .download_image(&image_link)
        .await?;

    let filename = format!("{}.jpg", random_string!(16));

    let attach = [Attachment::from_bytes(filename, image, 1337)];

    // Using it like this cuz there are always will be atleast one and only one attachment
    let source = ImageSource::attachment(&attach[0].filename)?; 

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
        .attachments(&attach)?
        .await?;

    Ok(())
}

pub async fn twitch_checker(ctx: &FumoContext) -> Result<()> {
    let data_db = ctx.db.get_streamers().await?;

    let names: Vec<i64> = data_db.iter()
        .map(|s: &TwitchTrackedStreamer| s.id)
        .collect();

    let data_api = match ctx.twitch_api.get_streams_by_id(
        names.as_slice()
    ).await? {
        Some(s) => s,
        None => bail!("Got None from twitch api")
    };

    for s_db in data_db {
        let online = s_db.online;

        match data_api.iter().find(|&i| i.user_id == s_db.id) {
            // If Some(_) that means that current streamer from database is online
            Some(s) => {
                if s.stream_type == StreamType::Live && !online {
                    ctx.db.toggle_online(s_db.id).await?;

                    let channels = ctx.db.get_channels_by_twitch_id(s_db.id).await?;
                    for channel in channels {
                        let channel_id: Id<ChannelMarker> = Id::new(channel.channel_id as u64);
                        announce_channel(ctx, channel_id, s).await?;
                    }
                }
            },
            // None means streamer from database is not currenly online
            None => {
                if online {
                    ctx.db.toggle_online(s_db.id).await?;
                }
            }
        }
    }

    Ok(())
}


async fn twitch_list(
    ctx: &FumoContext, 
    command: &InteractionCommand
) -> Result<()> {
    let channels = ctx.db.get_channels_by_channel_id(
        command.channel_id.get() as i64
    ).await?;

    let streamers = ctx.db.get_streamers()
        .await?;

    command.defer(ctx).await?;

    // Early exit just in case
    if channels.is_empty() {
        let builder = MessageBuilder::new()
            .content("Couldn't find any tracked twitch channels on current channel!");
        command.update(ctx, &builder).await?;
        return Ok(())
    };

    
    let mut display_list: Vec<i64> = Vec::new();
    for ch in channels {
        let s = match streamers.iter().find(|&x| x.id == ch.id) {
            Some(streamer) => streamer,
            None => bail!("Couldn't find twitch streamer???")
        };

        display_list.push(s.id)
    };
    
    // Getting users list from api to keep up with actual user name
    let api_streamers = ctx.twitch_api.get_users_by_id(&display_list)
        .await?
        .unwrap();

    let mut list = String::with_capacity(500);

    for s in api_streamers {
        let _  = writeln!(list, "{}", s.login);
    }

    let builder = MessageBuilder::new()
        .content(format!("```\n{list}```"));

    command.update(ctx, &builder).await?;

    Ok(())
}
/*

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

    let streamers = ctx.twitch_api.get_users_by_name(&[name])
        .await?
        .unwrap();

    // Checking if user with provided name actually exists
    let streamer = match streamers.get(0) {
        Some(s) => s,
        None => {
            msg = msg.content(
                format!("User with name `{name}` does not exists on twitch!")
                );
            command.update(ctx, &msg).await?;
            return Ok(());
        }
    };

    let streamer = match ctx.db.get_streamer(streamer.id).await {
        Some(s) => s,
        None => ctx.db.add_streamer(streamer.id).await?,
    };
    
    let channel_id: i64 = command.channel_id.get().try_into()?;
    match ctx.db.get_tracking(streamer.id, channel_id).await {
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

    let streamers = ctx.twitch_api.get_users_by_name(&[name])
        .await?
        .unwrap();

    let streamer = match streamers.get(0) {
        Some(s) => s,
        None => {
            msg = msg.content(format!(
                "User with name `{name}` does not exists on twitch!"
            ));
            command.update(ctx, &msg).await?;
            return Ok(());
        }
    };

    let streamer_db = match ctx.db.get_streamer_by_id(streamer.id).await {
        Some(s) => s,
        None => {
            msg = msg.content(format!(
                "`{name}` doesn't exists on current channel!"
            ));
            command.update(ctx, &msg).await?;
            return Ok(());
        }
    };
    
    if ctx.db.get_tracking(
        streamer_db.id, channel_id
    )
    .await
    .is_none() {
        msg = msg.content(format!(
            "`{name}` doesn't exists on current channel!"
        ));
        command.update(ctx, &msg).await?;
        return Ok(());
    }
    
    ctx.db.remove_tracking(streamer_db.id, channel_id).await?;

    msg = msg.content(
        format!("Successfully removed `{name}` from current channel!")
    );

    command.update(ctx, &msg).await?;
    Ok(())
}

pub async fn run(ctx: &FumoContext, command: InteractionCommand) -> Result<()> {
    if let Some(option) = &command.data.options.get(0) {
        match &option.value {
            CommandOptionValue::SubCommand(args) => {
                match option.name.as_ref() {
                    "add" => {
                        if let CommandOptionValue::String(name) = &args[0].value {
                            twitch_add(ctx, &command, name).await
                        } else { bail!("No required option provided!"); }
                    },
                    "remove" => { 
                        if let CommandOptionValue::String(name) = &args[0].value {
                            twitch_remove(ctx, &command, name).await
                        } else { bail!("No required option provided!"); }
                    },
                    "list" => twitch_list(ctx, &command).await,
                    &_ => bail!("Unrecognized option name `{}`", option.name),
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
