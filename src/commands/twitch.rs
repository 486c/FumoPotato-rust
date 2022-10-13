use std::time::Duration;
use std::sync::Arc;


use serenity::prelude::Context;
use serenity::model::prelude::interaction::application_command::ApplicationCommandInteraction;
use serenity::model::application::interaction::InteractionResponseType;

use crate::fumo_context::FumoContext;
use crate::database::twitch::TwitchChannel;

pub async fn announce_channel(ctx: &Context, c: &TwitchChannel) {
    let channel = match ctx.http.get_channel(c.id as u64).await {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Failed to recieve channel!");
            return;
        }
    };

    channel.id().send_message(&ctx.http, |m| {
        m.embed(|e| {
            e.author(|a| {
                a.name(format!("{} is live!", c.name))
                    .url(format!("https://twitch.tv/{}", c.name))
            })
            .color(0x97158a)
            .image(format!(
                "https://static-cdn.jtvnw.net/previews-ttv/live_user_{}-1280x720.jpg",
                c.name
            ))
        })
    }).await.unwrap();

}

pub async fn twitch_checker(ctx: Arc<Context>, fumo_ctx: Arc<FumoContext>) {
    loop {
        if let Ok(streamers) = fumo_ctx.db.get_streamers().await {
            for streamer_db in streamers.iter() {
                match fumo_ctx.twitch_api.get_stream(&streamer_db.name).await {
                    Some(s) => {
                        if s.stream_type == "live" && streamer_db.online == false {
                            fumo_ctx.db.toggle_online(&streamer_db.name).await; //TODO we should
                                                                                //handle error
                            if let Ok(channels) = fumo_ctx.db.get_channels(
                                &streamer_db.name
                            ).await 
                            {
                                for channel in channels.iter() {
                                    announce_channel(&ctx, channel).await;
                                }
                            }
                        }
                    }
                    None => {
                        if streamer_db.online == true {
                            fumo_ctx.db.toggle_online(&streamer_db.name).await; //TODO ^
                        }
                    }
                };
            }       
        } else {
            println!("failed to get streamers!");
        }

        tokio::time::sleep(Duration::from_secs(20)).await;
    }
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
    
    // Should never panic
    let streamer_name = command.data.options.get(0).unwrap().value.as_ref().unwrap().as_str().unwrap();
    let channel_id = command.channel_id.0.try_into().unwrap();

    let track = fumo_ctx.db.get_tracking(streamer_name, channel_id).await;

    match track {
        Some(_) => {
            match fumo_ctx.db.remove_tracking(streamer_name, channel_id).await {
                Ok(_) =>  {
                    command
                        .edit_original_interaction_response(&ctx.http, |m| {
                            m.content("Successfully removed user from current channel!") //TODO format name
                        })
                    .await.unwrap();
                }
                Err(_) => {
                    command
                        .edit_original_interaction_response(&ctx.http, |m| {
                            m.content("Can't remove user from current channel!") //TODO format name
                        })
                    .await.unwrap();
                    return;
                }
            }
        },
        None => {
            command
                .edit_original_interaction_response(&ctx.http, |m| {
                    m.content("User doesn't exists on current channel!") //TODO format name
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

    match fumo_ctx.db.get_tracking(&streamer.name, channel_id).await {
        Some(_) => {
            command
                .edit_original_interaction_response(&ctx.http, |m| {
                    m.content("User already added to current channel!") //TODO format name
                })
            .await.unwrap();
            return;
        },
        None => {
            match fumo_ctx.db.add_tracking(&streamer, channel_id).await {
                Ok(_) => {
                    command
                        .edit_original_interaction_response(&ctx.http, |m| {
                            m.content("Successfully added user to tracking!") //TODO format name
                        })
                    .await.unwrap();
                },
                Err(_) => {
                    command
                        .edit_original_interaction_response(&ctx.http, |m| {
                            m.content("Error occured during adding user to tracking!") //TODO format name
                        })
                    .await.unwrap();
                    return;
                }
            }
        }
    };
}
