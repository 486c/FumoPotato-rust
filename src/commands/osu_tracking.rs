use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::channel::message::MessageFlags;
use crate::{fumo_context::FumoContext, utils::{InteractionCommand, MessageBuilder}, osu_api::models::UserId};
use eyre::Result;

/// Osu tracking commands
#[derive(CommandModel, CreateCommand, Debug)]
#[command(name = "osu-tracking")]
pub enum OsuTracking {
    #[command(name = "add")]
    Add(OsuTrackingAdd)
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
