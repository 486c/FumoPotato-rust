mod commands;
mod config;
mod datetime;

pub mod osu_api;

use once_cell::sync::OnceCell;

use dotenv::dotenv;

use std::env;

use serenity::async_trait;
use serenity::model::application::command::{Command, CommandOptionType, CommandType};
use serenity::model::application::interaction::{ Interaction, InteractionType };
use serenity::model::gateway::Ready;
use serenity::prelude::*;

use config::BotConfig;
use osu_api::OsuApi;

static OSU_API: OnceCell<OsuApi> = OnceCell::new();

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            match command.data.name.to_lowercase().as_str() {
                "leaderboard" => commands::country_leaderboard::run(&ctx, &command).await,
                _ => (),
            };
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        Command::create_global_application_command(&ctx.http, |command| {
            command
                .name("leaderboard")
                .description("Show country leaderboard")
                .create_option(|option| {
                    option
                        .name("link")
                        .description("Direct link to beatmap")
                        .kind(CommandOptionType::String)
                        .required(true)
                })
        })
        .await.unwrap();

        Command::create_global_application_command(&ctx.http, |command| {
            command
                .name("Leaderboard")
                .kind(CommandType::Message)
        })
        .await.unwrap();
    }
}

#[tokio::main]
async fn main() {
    dotenv().unwrap();

    // Init config
    BotConfig::init();

    // Init osu_api helper
    let api = OsuApi::init(
        env::var("CLIENT_ID").unwrap().parse().unwrap(),
        env::var("CLIENT_SECRET").unwrap().as_str(),
    )
    .await
    .unwrap();

    OSU_API.set(api).unwrap();

    let token = env::var("DISCORD_TOKEN").unwrap();
    let mut client = Client::builder(token, GatewayIntents::empty())
        .event_handler(Handler)
        .await
        .expect("Error creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
