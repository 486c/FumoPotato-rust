use std::time::Duration;
use std::sync::Arc;

use serenity::prelude::Context;
use serenity::model::prelude::interaction::application_command::ApplicationCommandInteraction;
use serenity::model::application::interaction::InteractionResponseType;

use crate::database::Database;

pub async fn status_update_worker(ctx: Arc<Context>, db: Arc<Database>) {
    loop {
        tokio::time::sleep(Duration::from_secs(20)).await;
    }
}

pub async fn twitch_remove(ctx: &Context, command: &ApplicationCommandInteraction, db: &Database) {
    command
        .create_interaction_response(&ctx.http, |response| {
            response.kind(InteractionResponseType::DeferredChannelMessageWithSource)
        }).await.unwrap();
    
    // Should never panic
    let streamer_name = command.data.options.get(0).unwrap().value.as_ref().unwrap().as_str().unwrap();
    let channel_id = command.channel_id.0.try_into().unwrap();

    let track = db.get_tracking(streamer_name, channel_id).await;

    match track {
        Some(_) => {
            match db.remove_tracking(streamer_name, channel_id).await {
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

pub async fn twitch_add(ctx: &Context, command: &ApplicationCommandInteraction, db: &Database) {
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

    let streamer = match db.get_streamer(streamer_name).await {
        Some(s) => s,
        None => {
            match db.add_streamer(streamer_name).await {
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

    match db.get_tracking(&streamer.name, channel_id).await {
        Some(_) => {
            command
                .edit_original_interaction_response(&ctx.http, |m| {
                    m.content("User already added to current channel!") //TODO format name
                })
            .await.unwrap();
            return;
        },
        None => {
            match db.add_tracking(&streamer, channel_id).await {
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
