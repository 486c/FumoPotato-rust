use std::{sync::Arc, time::Duration};

use num_format::{Locale, ToFormattedString};
use tokio_stream::StreamExt;

use std::fmt::Write;

use chrono::Utc;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::application::interaction::{Interaction, InteractionData};
use twilight_model::{channel::message::MessageFlags, id::Id};
use twilight_util::builder::embed::{EmbedBuilder, EmbedFooterBuilder, ImageSource};
use crate::utils::{pages_components, InteractionComponent};
use crate::{fumo_context::FumoContext, utils::{InteractionCommand, MessageBuilder}, osu_api::models::{UserId, GetUserScores, ScoresType, GetRanking, OsuGameMode, RankingKind, RankingFilter}};
use eyre::Result;

macro_rules! osu_track_embed {
    ($score:expr, $user:expr) => {{
        let mut description_text = String::with_capacity(
            100
        );

        let _ = writeln!(
            description_text,
            "[**{} - {} [{}]**](https://osu.ppy.sh/b/{})",
            $score.beatmapset.artist,
            $score.beatmapset.title,
            $score.beatmap.version,
            $score.beatmap.id
        );

        let _ = writeln!(
            description_text,
            "**{} • +{} • {}**",
            $score.rank.to_emoji(),
            &$score.mods.to_string(),
            $score.score.to_formatted_string(&Locale::en)
        );

        let _ = writeln!(
            description_text,
            "**{:.2}pp** • <t:{}:R>",
            $score.pp.unwrap_or(0.0),
            $score.created_at.timestamp()
        );

        let _ = writeln!(
            description_text,
            "[{}/{}/{}/{}] • x{}",
            $score.stats.count300,
            $score.stats.count100,
            $score.stats.count50,
            $score.stats.countmiss,
            $score.max_combo.unwrap_or(0)
        );

        let thumb_url = format!(
            "https://b.ppy.sh/thumb/{}l.jpg",
            $score.beatmap.beatmapset_id
        );

        let mapper_name = &$score.beatmapset.creator;

        let footer = EmbedFooterBuilder::new(
            format!("Mapper {}", mapper_name)
        );

        EmbedBuilder::new()
            .color(0xbd49ff)
            .description(description_text)
            .footer(footer)
            .thumbnail(
                ImageSource::url(thumb_url).unwrap()
                )
            .title(
                format!(
                    "{} - {:.2} (#{})",
                    &$user.username,
                    $user.statistics.pp,
                    $user.statistics.global_rank,
                    )
                )
            .url(format!(
                    "https://osu.ppy.sh/u/{}",
                    $user.id
                ))
            .build()
    }}
}

pub async fn osu_track_checker(ctx: &FumoContext) {
        let mut lock = ctx.osu_checker_list.lock().await;

        for (osu_id, last_checked) in lock.iter_mut() {
            let now = Utc::now().naive_utc();

            let user_scores = ctx.osu_api.get_user_scores(
                GetUserScores::new(
                    *osu_id,
                    ScoresType::Best,
                ),
            ).await;

            if let Err(e) = &user_scores {
                println!("Error during osu_checker loop!");
                println!("{}", e); // TODO move to report
            }

            let user_scores = user_scores.unwrap();

            let linked_channels = 
                ctx.db.select_osu_tracked_linked_channels(
                    *osu_id
                ).await.unwrap(); // TODO remove

            for score in user_scores {
                if score.created_at.naive_utc() > *last_checked {
                    for c in &linked_channels {

                        let osu_user = ctx.osu_api.get_user(
                            UserId::Id(score.user_id),
                            None
                        ).await.unwrap().unwrap(); // TODO remove;

                        let embed = osu_track_embed!(score, osu_user);
                        

                        let _ = ctx.http.create_message(
                            Id::new(c.channel_id as u64)
                        ).embeds(&[embed]).unwrap().await;
                    }
                }
            }

            *last_checked = now;
        }

        drop(lock);
}

pub async fn osu_tracking_worker(ctx: Arc<FumoContext>) {
    println!("Syncing osu tracking list!");
    osu_sync_checker_list(&ctx).await.unwrap();
    
    println!("Starting osu tracking loop!");
    loop {
        osu_track_checker(&ctx).await;
        tokio::time::sleep(Duration::from_secs(360)).await;
    }
}

pub async fn osu_sync_checker_list(ctx: &FumoContext) -> Result<()> {
    let tracked_users = ctx.db.select_osu_tracked_users()
        .await?;

    let mut lock = ctx.osu_checker_list.lock().await;

    for tracked_user in tracked_users {
        let _ = lock.entry(tracked_user.osu_id)
            .or_insert(tracked_user.last_checked);
    }

    Ok(())
}

pub async fn osu_sync_db(ctx: Arc<FumoContext>) -> Result<()> {
    let lock = ctx.osu_checker_list.lock().await;

    for (osu_id, last_checked) in lock.iter() {
        ctx.db.update_tracked_user_status(*osu_id, *last_checked)
            .await?;
    };

    println!("Successfully synced db with osu tracked list");

    Ok(())
}

/// Osu tracking commands
#[derive(CommandModel, CreateCommand, Debug)]
#[command(name = "osu-tracking")]
pub enum OsuTracking {
    #[command(name = "add")]
    Add(OsuTrackingAdd),
    #[command(name = "remove")]
    Remove(OsuTrackingRemove),
    #[command(name = "add-bulk")]
    AddBulk(OsuTrackingAddBulk),
    #[command(name = "remove-all")]
    RemoveAll(OsuTrackingRemoveAll),
    #[command(name = "list")]
    List(OsuTrackingList),
}

impl OsuTracking {
    pub async fn handle(
        ctx: &FumoContext, 
        cmd: InteractionCommand
    ) -> Result<()> {
        let command = Self::from_interaction(
            cmd.data.clone().into()
        )?;

        match command {
            OsuTracking::Add(command) => command.run(ctx, cmd).await,
            OsuTracking::Remove(command) => command.run(ctx, cmd).await,
            OsuTracking::AddBulk(command) => command.run(ctx, cmd).await,
            OsuTracking::RemoveAll(command) => command.run(ctx, cmd).await,
            OsuTracking::List(command) => command.run(ctx, cmd).await,
        }
    }
}

/// Remove osu user from tracking
#[derive(CommandModel, CreateCommand, Debug)]
#[command(name = "remove")]
pub struct OsuTrackingRemove {
    /// osu! username or user id
    osu_user: String
}

impl OsuTrackingRemove {
    pub async fn run(
        &self,
        ctx: &FumoContext,
        cmd: InteractionCommand
    ) -> Result<()> {
        let channel_id = cmd.channel_id.get().try_into()?;

        let osu_user = ctx.osu_api.get_user(
            UserId::Username(self.osu_user.clone()), // TODO avoid stupid clones
            None,
        ).await?;

        let mut msg = MessageBuilder::new()
            .flags(MessageFlags::EPHEMERAL);

        if osu_user.is_none() {
            msg = msg.content("User not found!");
            cmd.response(ctx, &msg).await?;
            return Ok(());
        }

        let osu_user = osu_user.unwrap();

        let osu_tracked = ctx.db.select_osu_tracking(
            channel_id,
            osu_user.id,
        ).await?;

        match osu_tracked {
            Some(_) => {
                ctx.db.remove_osu_tracking(
                    channel_id,
                    osu_user.id
                ).await?;

                msg = msg.content(
                    "Successfully removed user from tracking"
                );

                cmd.response(ctx, &msg).await?;
            },
            None => {
                msg = msg.content(
                    "This user is not currently tracked on this channel!"
                );

                cmd.response(ctx, &msg).await?;
            },
        }

        Ok(())

    }
}

/// Add osu user to the tracking
#[derive(CommandModel, CreateCommand, Debug)]
#[command(name = "add")]
pub struct OsuTrackingAdd {
    /// osu! username or user id
    osu_user: String
}

impl OsuTrackingAdd {
    pub async fn run(
        &self,
        ctx: &FumoContext,
        cmd: InteractionCommand
    ) -> Result<()> {
        let osu_user = ctx.osu_api.get_user(
            UserId::Username(self.osu_user.clone()), // TODO avoid stupid clones
            None,
        ).await?;

        let channel_id = cmd.channel_id.get().try_into()?;

        let mut msg = MessageBuilder::new()
            .flags(MessageFlags::EPHEMERAL)
            .content("User not found!");

        match osu_user {
            Some(osu_user) => {
                // Check if user is already tracked
                let osu_tracked = ctx.db.select_osu_tracking(
                    channel_id,
                    osu_user.id,
                ).await?;

                match osu_tracked {
                    Some(_) => {
                        msg = msg.content("User is already tracked");
                        cmd.response(ctx, &msg).await?;
                        Ok(())
                    },
                    None => {
                        add_osu_tracking_user!(
                            ctx, 
                            &osu_user, 
                            channel_id
                        );

                        osu_sync_checker_list(ctx).await?;

                        msg = msg.content(
                            "Successfully added user to the tracking!"
                        );
                        cmd.response(ctx, &msg).await?;
                        Ok(())
                    },
                }
            },
            None => {
                msg = msg.content("User not found!");
                cmd.response(ctx, &msg).await?;

                Ok(())
            },
        }
    }
}

#[derive(CommandModel, CreateCommand, Debug)]
#[command(name = "add-bulk", desc="
    Add multiple users to the tracking, either based on country 
    or global leaderboards
")]
pub struct OsuTrackingAddBulk {
    /// Amount of users to add
    #[command(min_value=1, max_value=50)]
    amount: i64,

    /// Country code, if not specified then global leaderboard
    /// is going to be used
    #[command(min_length=2, max_length=2)]
    country: Option<String>,

    /// Starting page (1 page = 50 players)
    #[command(min_value=1, max_value=10)]
    page: Option<i64>
}

impl OsuTrackingAddBulk {
    pub async fn run(
        &self,
        ctx: &FumoContext,
        cmd: InteractionCommand
    ) -> Result<()> {
        let channel_id = cmd.channel_id.get().try_into()?;

        ctx.db.add_discord_channel(channel_id).await?;

        // Fetch all tracked users in current channel
        let tracked_users = ctx.db.select_osu_tracking_by_channel(
            channel_id,
        ).await?;

        let (ranking_kind, country_code) = match &self.country {
            Some(country_code) => {
                (RankingKind::Performance, Some(country_code.clone()))
            },
            None => {
                (RankingKind::Performance, None)
            },
        };

        let page = self.page.unwrap_or(0);

        let get_ranking = GetRanking {
            mode: OsuGameMode::Osu,
            kind: ranking_kind,
            filter: RankingFilter::All,
            country: country_code,
            page: Some(page as u32) 
        };

        // Fetch users that should be added
        let rankings = ctx.osu_api.get_rankings(
            &get_ranking,
            self.amount as usize,
        ).await?;

        let mut str = String::new();

        let _ = writeln!(str, "```");

        for stats in rankings.ranking {
            // TODO lmao wtf is this refactor ASAP
            if tracked_users.iter().any(|x| x.osu_id == stats.user.id) {
                let _ = writeln!(
                    str, 
                    "{} - Already tracked", 
                    stats.user.username
                );
            } else {
                ctx.db.add_osu_player(
                    stats.user.id,
                    &stats.user.username
                ).await?;

                ctx.db.add_tracked_osu_user(
                    stats.user.id
                ).await?;

                ctx.db.add_osu_tracking(
                    channel_id,
                    stats.user.id
                ).await?;

                let _ = writeln!(
                    str, 
                    "{} - Added", 
                    stats.user.username
                );
            }
        };

        let _ = writeln!(str, "```");

        let msg = MessageBuilder::new()
            .flags(MessageFlags::EPHEMERAL)
            .content(str);

        cmd.response(ctx, &msg).await?;

        Ok(())
    }
}

#[derive(CommandModel, CreateCommand, Debug)]
#[command(name = "remove-all", desc="
    Remove all tracked users from current channel
")]
pub struct OsuTrackingRemoveAll {}

impl OsuTrackingRemoveAll {
    pub async fn run(
        &self,
        ctx: &FumoContext,
        cmd: InteractionCommand
    ) -> Result<()> {
        let channel_id: i64 = cmd.channel_id.get().try_into()?;

        ctx.db.remove_all_osu_tracking(channel_id).await?;

        let msg = MessageBuilder::new()
            .flags(MessageFlags::EPHEMERAL)
            .content("
                Successfully removed all tracked users from current channel"
            );

        cmd.response(ctx, &msg).await?;

        Ok(())
    }
}

#[derive(CommandModel, CreateCommand, Debug)]
#[command(name = "list", desc="
    List all tracked users on current channel
")]
pub struct OsuTrackingList {}

impl OsuTrackingList {
    pub async fn run(
        &self,
        ctx: &FumoContext,
        cmd: InteractionCommand
    ) -> Result<()> {
        cmd.defer(ctx).await?;

        let channel_id: i64 = cmd.channel_id.get().try_into()?;

        let tracked_users = ctx.db.select_osu_tracking_by_channel(
            channel_id,
        ).await?;

        let elem_per_page = 10;

        let pages: u32 = (
            tracked_users.len() as f32 / elem_per_page as f32
        ).ceil() as u32;

        let mut current_page = 1;

        let footer_text = format!(
            "Tracked users: {} • Page: {}/{}", 
            tracked_users.len(), current_page, pages
        );

        let mut body_text = String::with_capacity(100);

        for tracked_user in tracked_users.iter()
            .take(elem_per_page as usize)
            {
                let _ = writeln!(
                    body_text, 
                    "{}", 
                    &tracked_user.osu_username
                    );
            }

        let embed = EmbedBuilder::new()
            .color(0xbd49ff)
            .title("Tracked users")
            .footer(
                EmbedFooterBuilder::new(
                    footer_text
                )
            )
            .description(body_text)
            .build();

        let mut msg_builder = MessageBuilder::new();

        msg_builder.embed = Some(embed);
        msg_builder.components = Some(pages_components());

        let msg = cmd.update(ctx, &msg_builder).await?
            .model().await?;

        let stream = component_stream!(ctx, msg);

        tokio::pin!(stream);

        while let Some(Ok(component)) = stream.next().await {
            if let Some(data) = &component.data {
                match data.custom_id.as_ref() {
                    "B1" => current_page = (current_page - 1).max(1),
                    "B2" => current_page = (current_page + 1).min(pages),
                    _ => {},
                }
            } 

            let start_at = (current_page-1)*elem_per_page;

            let embed = &mut msg_builder.embed;
            
            component.defer(ctx).await?;

            // Update body
            if let Some(embed) = embed {
                if let Some(description) = &mut embed.description {
                    description.clear();
                    for tracked_user in tracked_users.iter()
                        .skip(start_at as usize)
                        .take(elem_per_page as usize)
                    {
                        let _ = writeln!(
                            description, 
                            "{}", 
                            &tracked_user.osu_username
                        );
                    }
                }

                if let Some(footer) = &mut embed.footer {
                    footer.text.clear();

                    let _ = write!(
                        footer.text,
                        "Tracked users: {} • Page: {}/{}", 
                        tracked_users.len(), current_page, pages
                    );
                }
            }

            cmd.update(ctx, &msg_builder).await?;
        }

        if let Some(components) = &mut msg_builder.components {
            components.clear();
        }

        cmd.update(ctx, &msg_builder).await?;

        Ok(())
    }
}
