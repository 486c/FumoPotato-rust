use std::{sync::Arc, time::Duration};

use std::fmt::Write;

use chrono::Utc;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::{channel::message::MessageFlags, id::Id};
use crate::{fumo_context::FumoContext, utils::{InteractionCommand, MessageBuilder}, osu_api::models::{UserId, GetUserScores, ScoresType, GetRanking, OsuGameMode, RankingKind, RankingFilter}};
use eyre::Result;

pub async fn osu_track_checker(ctx: &FumoContext) {
        let mut lock = ctx.osu_checker_list.lock().await;

        for (osu_id, last_checked) in lock.iter_mut() {
            let now = Utc::now().naive_utc();

            let user_scores = ctx.osu_api.get_user_scores(
                GetUserScores::new(
                    *osu_id,
                    ScoresType::Best,
                ),
            ).await.unwrap(); // TODO remove

            let linked_channels = 
                ctx.db.select_osu_tracked_linked_channels(
                    *osu_id
                ).await.unwrap(); // TODO remove

            for score in user_scores {
                if score.created_at.naive_utc() > *last_checked {
                    for c in &linked_channels {
                        let _ = ctx.http.create_message(
                            Id::new(c.channel_id as u64)
                        ).content(
                            &format!(
                                "New top score {}pp by `{}`",
                                score.pp.unwrap_or(0.0),
                                &score.user.username
                            )
                        ).unwrap().await;
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
    AddBulk(OsuTrackingAddBulk)
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
            OsuTracking::Add(command) => command.run(&ctx, cmd).await,
            OsuTracking::Remove(command) => command.run(&ctx, cmd).await,
            OsuTracking::AddBulk(command) => command.run(&ctx, cmd).await,
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
                        return Ok(());
                    },
                    None => {
                        add_osu_tracking_user!(
                            ctx, 
                            osu_user.id, 
                            channel_id
                        );

                        osu_sync_checker_list(&ctx).await?;

                        msg = msg.content(
                            "Successfully added user to the tracking!"
                        );
                        cmd.response(ctx, &msg).await?;
                        return Ok(());
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

/// Add multiple users to the tracking, either based on country
/// or global leaderboards
#[derive(CommandModel, CreateCommand, Debug)]
#[command(name = "add-bulk")]
pub struct OsuTrackingAddBulk {
    /// Amount of users to add
    #[command(min_value=1, max_value=50)]
    amount: i64,

    /// Country code, if not specified then global leaderboard
    /// is going to be used
    #[command(min_length=2, max_length=2)]
    country: Option<String>,
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

        let get_ranking = GetRanking {
            mode: OsuGameMode::Osu,
            kind: ranking_kind,
            filter: RankingFilter::All,
            country: country_code
        };

        // Fetch users that should be added
        let rankings = ctx.osu_api.get_rankings(
            &get_ranking,
            self.amount as usize,
        ).await?;

        let mut str = String::new();

        let _ = writeln!(str, "```");

        for stats in rankings.ranking {
            // TODO lmao wtf is this 
            if tracked_users.iter().any(|x| x.osu_id == stats.user.id) {
                let _ = writeln!(
                    str, 
                    "{} - Already tracked", 
                    stats.user.username
                );
            } else {
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

