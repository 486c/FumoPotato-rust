use std::time::Duration;
use std::sync::Arc;

use serenity::prelude::Context;
use serenity::http::client::Http;
use serenity::model::prelude::interaction::application_command::ApplicationCommandInteraction;
use serenity::model::application::interaction::InteractionResponseType;
use serenity::model::channel::Channel;

use crate::fumo_context::FumoContext;
use crate::twitch_api::TwitchStream;

use anyhow::Result;

pub async fn announce_channel(http: &Http, channel: Channel, c: &TwitchStream) -> Result<()> {
    channel.id().send_message(&http, |m| {
        m.embed(|e| {
            e.author(|a| {
                a.name(format!("{} is live!", &c.user_name))
                    .url(format!("https://twitch.tv/{}", &c.user_name))
            })
            .description(&c.title)
            .color(0x97158a)
            .image(format!(
                "https://static-cdn.jtvnw.net/previews-ttv/live_user_{}-1280x720.jpg",
                &c.user_name
            ))

            .footer(|f| {
                f.text(&c.game_name);
                f.icon_url(format!(
                    "https://static-cdn.jtvnw.net/ttv-boxart/{}-250x250.jpg",
                    c.game_id
                ))
            })
        })
    }).await?;

    Ok(())
}

pub async fn twitch_worker(http: Arc<Http>, fumo_ctx: Arc<FumoContext>) {
    loop {
        match twitch_check(&http, &fumo_ctx).await {
            Ok(_) => (),
            Err(e) => {
                println!("Error occured inside twitch tracking loop!");
                println!("{:?}", e);
            }
        }
        tokio::time::sleep(Duration::from_secs(120)).await;
    }
}

pub async fn twitch_check(http: &Http, fumo_ctx: &FumoContext) -> Result<()> {
    let streamers  = fumo_ctx.db.get_streamers().await?;

    for streamer_db in streamers.iter() {
        let name = &streamer_db.name;
        let online = streamer_db.online;

        match fumo_ctx.twitch_api.get_stream(name).await {
            Some(streamer) => {
                if streamer.stream_type == "live" && !online {

                    fumo_ctx.db.toggle_online(name).await?;

                    let channels = fumo_ctx.db.get_channels(name).await?;

                    for channel in channels.iter() {
                        let channel_id = http.get_channel(channel.id as u64).await?;
                        announce_channel(&http, channel_id, &streamer).await?;
                    }
            }
            }
            None => {
                fumo_ctx.db.toggle_online(&streamer_db.name).await?;
            }
        }
    }       

    Ok(())
}

pub async fn twitch_remove(
    ctx: &Context, 
    fumo_ctx: &FumoContext, 
    command: &ApplicationCommandInteraction
) {
    command
        .create_interaction_response(&ctx.http, |response| {
            response.kind(InteractionResponseType::DeferredChannelMessageWithSource)
        }).await.unwrap();
    
    // Should never panic //TODO so ugly lmao
    let streamer_name = command.data.options.get(0).unwrap()
        .value.as_ref().unwrap()
        .as_str().unwrap();

    let channel_id = command.channel_id.0.try_into().unwrap();

    let track = fumo_ctx.db.get_tracking(streamer_name, channel_id).await;

    match track {
        Some(_) => {
            match fumo_ctx.db.remove_tracking(streamer_name, channel_id).await {
                Ok(_) =>  {
                    command
                        .edit_original_interaction_response(&ctx.http, |m| {
                            m.content(
                                format!("Successfully removed `{}` from current channel!",
                                       streamer_name)
                            )
                        })
                    .await.unwrap();
                }
                Err(_) => {
                    command
                        .edit_original_interaction_response(&ctx.http, |m| {
                            m.content(
                                format!("Can't remove `{}` from current channel!",
                                        streamer_name)
                            ) 
                        })
                    .await.unwrap();
                }
            }
        },
        None => {
            command
                .edit_original_interaction_response(&ctx.http, |m| {
                    m.content(
                        format!("`{}` doesn't exists on current channel!",
                            streamer_name)
                    )
                })
            .await.unwrap();
        }
    };
}

pub async fn twitch_add(
    ctx: &Context, 
    fumo_ctx: &FumoContext, 
    command: &ApplicationCommandInteraction
) {
    command
        .create_interaction_response(&ctx.http, |response| {
            response.kind(InteractionResponseType::DeferredChannelMessageWithSource)
        }).await.unwrap();

    let streamer_name = match command.data.options.get(0) {
        Some(option) => option,
        None => return, // TODO Send message with error
    };

    let streamer_name = match streamer_name.value.as_ref() {
        Some(value) => value,
        None => return, // TODO send message error
    };

    let streamer_name = match streamer_name.as_str() {
        Some(name) => name,
        None => return,
    };

    let streamer = match fumo_ctx.db.get_streamer(streamer_name).await {
        Some(s) => s,
        None => {
            match fumo_ctx.db.add_streamer(streamer_name).await {
                Ok(streamer) => streamer,
                Err(_) => {
                    command
                        .edit_original_interaction_response(&ctx.http, |m| {
                            m.content("Error occured during adding streamer to database!")
                        })
                    .await.unwrap();
                    return;
                }
            }
        }, 
    };

    let channel_id: i64 = command.channel_id.0.try_into().unwrap();

    match fumo_ctx.db.get_tracking(&streamer_name, channel_id).await {
        Some(_) => {
            command
                .edit_original_interaction_response(&ctx.http, |m| {
                    m.content(format!(
                        "`{}` already added to current channel!",
                        &streamer_name
                    ))
                })
            .await.unwrap();
        },
        None => {
            match fumo_ctx.db.add_tracking(&streamer, channel_id).await {
                Ok(_) => {
                    command
                        .edit_original_interaction_response(&ctx.http, |m| {
                            m.content(format!(
                                "Successfully added `{}` to tracking!",
                                &streamer_name
                            ))
                        })
                    .await.unwrap();
                },
                Err(_) => {
                    command
                        .edit_original_interaction_response(&ctx.http, |m| {
                            m.content(format!(
                                "Error occured during adding `{}` to tracking!",
                                &streamer_name
                            ))
                        })
                    .await.unwrap();
                }
            }
        }
    };
}
