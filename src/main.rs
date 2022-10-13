mod commands;
mod config;
mod datetime;

pub mod database;

pub mod osu_api;
pub mod twitch_api;

use once_cell::sync::OnceCell;

use std::sync::Arc;

use dotenv::dotenv;

use std::env;

use serenity::async_trait;
use serenity::model::application::command::{Command, CommandOptionType, CommandType};
use serenity::model::application::interaction::Interaction;
use serenity::model::gateway::Ready;
use std::sync::atomic::{AtomicBool, Ordering};
use serenity::prelude::*;

use config::BotConfig;

use osu_api::OsuApi;
use twitch_api::TwitchApi;

static OSU_API: OnceCell<OsuApi> = OnceCell::new();

struct Handler {
    is_twitch_loop_running: AtomicBool,
    db: Arc<database::Database>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            match command.data.name.to_lowercase().as_str() {
                "leaderboard" => commands::country_leaderboard::run(&ctx, &command).await,
                "twitch_add" => commands::twitch::twitch_add(&ctx, &command, &self.db).await,
                "twitch_remove" => commands::twitch::twitch_remove(&ctx, &command, &self.db).await,
                _ => (),
            };
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        Command::set_global_application_commands(&ctx.http, |commands| {
            commands
                .create_application_command(|c| {
                    c.name("leaderboard")
                        .description("Show country leaderboard")
                        .create_option(|option| {
                            option
                                .name("link")
                                .description("Direct link to beatmap")
                                .kind(CommandOptionType::String)
                                .required(false)
                        })
                })
                .create_application_command(|c| {
                    c.name("Leaderboard")
                        .kind(CommandType::Message)
                })
                .create_application_command(|c| {
                    c.name("twitch_add")
                        .create_option(|option| {
                            option
                                .name("name")
                                .description("Twitch channel")
                                .kind(CommandOptionType::String)
                                .required(true)
                        })
                })
                .create_application_command(|c| {
                    c.name("twitch remove")
                        .create_option(|option| {
                            option
                                .name("name")
                                .description("Twitch channel")
                                .kind(CommandOptionType::String)
                                .required(true)
                        })
                })
        }).await.unwrap();

        let ctx = Arc::new(ctx);
        let db = Arc::clone(&self.db);

        if !self.is_twitch_loop_running.load(Ordering::Relaxed) {
            tokio::spawn(async move {
                commands::twitch::status_update_worker(
                    Arc::clone(&ctx), Arc::clone(&db)
                ).await;
            });
        }

        self.is_twitch_loop_running.swap(true, Ordering::Relaxed);
    }
}

#[tokio::main]
async fn main() {
    dotenv().unwrap();

    // TODO make one big context that we pass to all commands 

    // Init config
    BotConfig::init();

    // Init twitch api
    let twitch_api = TwitchApi::init(
        env::var("TWITCH_TOKEN").unwrap().as_str(),
        env::var("TWITCH_CLIENT_ID").unwrap().as_str()
    ).await.unwrap();

    // Init osu_api helper
    let api = OsuApi::init(
        env::var("CLIENT_ID").unwrap().parse().unwrap(),
        env::var("CLIENT_SECRET").unwrap().as_str(),
    ).await.unwrap();

    let db = database::Database::init(
        env::var("DATABASE_URL").unwrap().as_str()
    ).await.unwrap();

    let db = Arc::new(db);

    //TODO dirty way to pass osu_api handler, please consider changing this later...
    OSU_API.set(api).unwrap();

    let token = env::var("DISCORD_TOKEN").unwrap();

    let mut client = Client::builder(token, GatewayIntents::empty())
        .event_handler(Handler {
            is_twitch_loop_running: AtomicBool::new(false),
            db
        })
        .await
        .expect("Error creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
