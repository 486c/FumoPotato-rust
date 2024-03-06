use std::{sync::Arc, time::Duration};

use chrono::Utc;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::{channel::message::MessageFlags, id::Id};
use crate::{fumo_context::FumoContext, utils::{InteractionCommand, MessageBuilder}, osu_api::models::{UserId, GetUserScores, ScoresType}};
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
    Remove(OsuTrackingRemove)
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
                    "Successfully remove user from tracking"
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
