use std::{fmt::Write, slice, sync::Arc, time::Duration};

use fumo_twilight::message::MessageBuilder;
use twilight_model::{
    application::interaction::application_command::CommandOptionValue,
    http::attachment::Attachment,
    id::{marker::ChannelMarker, Id},
};
use twilight_util::builder::embed::{
    image_source::ImageSource, EmbedAuthorBuilder, EmbedBuilder,
    EmbedFooterBuilder,
};

use rand::distributions::{Alphanumeric, DistString};

use crate::{
    fumo_context::FumoContext,
    twitch_api::{StreamType, TwitchStream},
    utils::InteractionCommand,
};

use crate::random_string;

use eyre::{bail, Result};

pub async fn twitch_worker(ctx: Arc<FumoContext>) {
    println!("Syncing twitch checker list!");
    twitch_sync_checker_list(&ctx).await.unwrap();

    println!("Starting twitch checker loop!");
    loop {
        if let Err(e) = twitch_checker(&ctx).await {
            println!(
                "{:?}",
                e.wrap_err("Error occured inside twitch tracking loop!")
            );
        }
        tokio::time::sleep(Duration::from_secs(120)).await;
    }
}

pub async fn announce_channel(
    ctx: &FumoContext,
    channel_id: Id<ChannelMarker>,
    c: &TwitchStream,
) -> Result<()> {
    let author = EmbedAuthorBuilder::new(format!("{} is live!", &c.user_name))
        .url(format!("https://twitch.tv/{}", &c.user_login))
        .build();

    let image_link = format!(
        "https://static-cdn.jtvnw.net/previews-ttv/live_user_{}-1280x720.jpg",
        &c.user_login
    );

    let image = ctx.twitch_api.download_image(&image_link).await?;

    let filename = format!("{}.jpg", random_string!(16));

    let attach = [Attachment::from_bytes(filename, image, 1337)];

    // Using it like this cuz there are always will be atleast
    // one and only one attachment
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

    ctx.http
        .create_message(channel_id)
        .embeds(slice::from_ref(&embed))?
        .attachments(&attach)?
        .await?;

    Ok(())
}

/// Syncing database with checker list
pub async fn twitch_sync_checker_list(ctx: &FumoContext) -> Result<()> {
    let streamers = ctx.db.get_streamers().await?;

    let mut lock = ctx.twitch_checker_list.lock().await;

    for streamer in streamers {
        let _ = lock.entry(streamer.twitch_id).or_insert(streamer.online);
    }

    Ok(())
}

/// Syncing checker list with database
/// Happens on shutdown
pub async fn twitch_sync_db(ctx: Arc<FumoContext>) -> Result<()> {
    let lock = ctx.twitch_checker_list.lock().await;

    for (streamer_id, status) in lock.iter() {
        ctx.db.set_online_status(*streamer_id, *status).await?;
    }

    println!("Database is successfully synced with checker list");
    Ok(())
}

pub async fn twitch_checker(ctx: &FumoContext) -> Result<()> {
    // Taking lock of currently tracked streamers
    let mut tracked_list = ctx.twitch_checker_list.lock().await;

    // Fetching current status of all selected streamers
    let ids_to_fetch: Vec<i64> = tracked_list.keys().copied().collect();

    if ids_to_fetch.is_empty() {
        return Ok(());
    }

    let fetched_streamers = match ctx
        .twitch_api
        .get_streams_by_id(ids_to_fetch.as_slice())
        .await?
    {
        Some(s) => s,
        None => bail!("Got None from twitch api"),
    };

    for (id, is_online) in tracked_list.iter_mut() {
        let streamer_status = 'blk: {
            for twitch_streamer in &fetched_streamers {
                if twitch_streamer.user_id == *id {
                    break 'blk Some(twitch_streamer);
                }
            }

            break 'blk None;
        };

        match streamer_status {
            Some(streamer_status) => {
                // If currently offline streamer goes online
                if streamer_status.stream_type == StreamType::Live
                    && !(*is_online)
                {
                    *is_online = true;

                    // TODO move channel handling into announce function
                    // and also spawn another task for that instead
                    // of keeping lock forever
                    let channels =
                        ctx.db.get_channels_by_twitch_id(*id).await?;

                    for channel in channels {
                        let channel_id: Id<ChannelMarker> =
                            Id::new(channel.channel_id as u64);

                        let res =
                            announce_channel(ctx, channel_id, streamer_status)
                                .await;

                        if let Err(e) = res {
                            println!("Error happened during announcing");

                            println!("{:?}", e);
                        }
                    }
                }

                // If currently online streamer goes offline
                if streamer_status.stream_type == StreamType::Offline
                    && *is_online
                {
                    *is_online = false;
                }
            }
            // None returned means it's probably offline
            None => {
                if *is_online {
                    *is_online = false;
                }
            }
        }
    }

    Ok(())
}

async fn twitch_list(
    ctx: &FumoContext,
    command: &InteractionCommand,
) -> Result<()> {
    let channels = ctx
        .db
        .get_channels_by_channel_id(command.channel_id.get() as i64)
        .await?;

    let streamers = ctx.db.get_streamers().await?;

    command.defer(ctx).await?;

    // Early exit just in case
    if channels.is_empty() {
        let builder = MessageBuilder::new().content(
            "Couldn't find any tracked twitch channels on current channel!",
        );
        command.update(ctx, &builder).await?;
        return Ok(());
    };

    let mut display_list: Vec<i64> = Vec::new();
    for ch in channels {
        let s = match streamers.iter().find(|&x| x.twitch_id == ch.twitch_id) {
            Some(streamer) => streamer,
            None => bail!("Couldn't find twitch streamer???"),
        };

        display_list.push(s.twitch_id)
    }

    // Getting users list from api to keep up with actual user name
    let api_streamers = ctx
        .twitch_api
        .get_users_by_id(&display_list)
        .await?
        .unwrap();

    let mut list = String::with_capacity(500);

    for s in api_streamers {
        let _ = writeln!(list, "{}", s.login);
    }

    let builder = MessageBuilder::new().content(format!("```\n{list}```"));

    command.update(ctx, &builder).await?;

    Ok(())
}

async fn twitch_add(
    ctx: &FumoContext,
    command: &InteractionCommand,
    name: &str,
) -> Result<()> {
    command.defer(ctx).await?;
    let mut msg = MessageBuilder::new();

    let streamers = ctx.twitch_api.get_users_by_name(&[name]).await?.unwrap();

    // Checking if user with provided name actually exists
    let streamer = match streamers.first() {
        Some(s) => s,
        None => {
            msg = msg.content(format!(
                "User with name `{name}` does not exists on twitch!"
            ));

            command.update(ctx, &msg).await?;
            return Ok(());
        }
    };

    let streamer = match ctx.db.get_streamer(streamer.id).await {
        Some(s) => s,
        None => ctx.db.add_streamer(streamer.id).await?,
    };

    let channel_id: i64 = command.channel_id.get().try_into()?;
    match ctx.db.get_tracking(streamer.twitch_id, channel_id).await {
        Some(_) => {
            msg = msg
                .content(format!("`{name}` already added to current channel!"));
            command.update(ctx, &msg).await?;
            Ok(())
        }
        None => {
            ctx.db.add_tracking(&streamer, channel_id).await?;
            msg = msg
                .content(format!("Successfully added `{name}` to tracking!"));
            command.update(ctx, &msg).await?;

            twitch_sync_checker_list(ctx).await?;

            Ok(())
        }
    }
}

async fn twitch_remove(
    ctx: &FumoContext,
    command: &InteractionCommand,
    name: &str,
) -> Result<()> {
    command.defer(ctx).await?;
    let mut msg = MessageBuilder::new();

    let channel_id: i64 = command.channel_id.get().try_into()?;

    let streamers = ctx.twitch_api.get_users_by_name(&[name]).await?.unwrap();

    let streamer = match streamers.first() {
        Some(s) => s,
        None => {
            msg = msg.content(format!(
                "User with name `{name}` does not exists on twitch!"
            ));
            command.update(ctx, &msg).await?;
            return Ok(());
        }
    };

    let streamer_db = match ctx.db.get_streamer(streamer.id).await {
        Some(s) => s,
        None => {
            msg = msg.content(format!(
                "`{name}` doesn't exists on current channel!"
            ));
            command.update(ctx, &msg).await?;
            return Ok(());
        }
    };

    if ctx
        .db
        .get_tracking(streamer_db.twitch_id, channel_id)
        .await
        .is_none()
    {
        msg =
            msg.content(format!("`{name}` doesn't exists on current channel!"));
        command.update(ctx, &msg).await?;
        return Ok(());
    }

    ctx.db
        .remove_tracking(streamer_db.twitch_id, channel_id)
        .await?;

    msg = msg.content(format!(
        "Successfully removed `{name}` from current channel!"
    ));

    command.update(ctx, &msg).await?;
    Ok(())
}

pub async fn run(ctx: &FumoContext, command: InteractionCommand) -> Result<()> {
    if let Some(option) = command.data.options.first() {
        if let CommandOptionValue::SubCommand(args) = &option.value {
            match option.name.as_ref() {
                "add" => {
                    if let CommandOptionValue::String(name) = &args[0].value {
                        twitch_add(ctx, &command, name).await
                    } else {
                        bail!("No required option provided!");
                    }
                }
                "remove" => {
                    if let CommandOptionValue::String(name) = &args[0].value {
                        twitch_remove(ctx, &command, name).await
                    } else {
                        bail!("No required option provided!");
                    }
                }
                "list" => twitch_list(ctx, &command).await,
                _ => todo!(),
            }
        } else {
            bail!("No subcommand found")
        }
    } else {
        bail!("Required option is not found")
    }
}
