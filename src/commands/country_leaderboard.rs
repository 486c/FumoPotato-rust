use crate::osu_api::models::{OsuBeatmap, OsuScore, RankStatus};
use crate::fumo_context::FumoContext;
use crate::handlers::InteractionCommand;

use twilight_model::application::interaction::application_command::{
    CommandOptionValue::String
};

use twilight_util::builder::InteractionResponseDataBuilder;
use twilight_model::http::interaction::InteractionResponseData;
use twilight_model::channel::message::MessageFlags;
use twilight_model::http::interaction::InteractionResponseType;

fn parse_link(str: &str) -> Option<i32> {
    //TODO rewrite this shit xD
    let split: Vec<&str> = str.split('/').collect();

    // if full beatmapset link
    if split.len() == 6 {
        // Should never fail
        return Some(split.get(5).unwrap().parse::<i32>().unwrap());
    }

    // if compact link to beatmap
    // aka /b/id & /beatmaps/id
    if split.len() == 5 {
        // Also Should never fail
        return Some(split.get(4).unwrap().parse::<i32>().unwrap());
    }

    None
}

fn send_fail(text: &str) -> InteractionResponseData {
    InteractionResponseDataBuilder::new()
        .content(text)
        .flags(MessageFlags::EPHEMERAL)
        .build()
}

pub async fn run(ctx: &FumoContext, command: InteractionCommand) {

    command.create_response(
        &ctx, 
        &send_fail("test"),
        InteractionResponseType::ChannelMessageWithSource
    ).await.unwrap();

    // If link to beatmap is provided as argument
    /*
    if let Some(option) = command.data.options.get(0) {
        if let String(link) = &option.value {
            match parse_link(&link) {
                Some(bid) => {},
                None => {
                    
                }
            }
            
        }
    }
    println!("probably command");
    */
}
