use eyre::Result;
use fumo_twilight::message::MessageBuilder;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::channel::message::MessageFlags;

use crate::{fumo_context::FumoContext, utils::interaction::InteractionCommand};

use osu_api::models::UserId;

use super::{attributes::OsuAttributes, osu_tracking::OsuTracking};

/// All osu! related commands
#[derive(CommandModel, CreateCommand, Debug)]
#[command(name = "osu")]
pub enum OsuCommands {
    #[command(name = "link")]
    Link(OsuLink),
    #[command(name = "unlink")]
    Unlink(OsuUnlink),
    #[command(name = "attributes")]
    Attributes(OsuAttributes),
    #[command(name = "tracking")]
    Tracking(OsuTracking),
}

impl OsuCommands {
    pub async fn handle(
        ctx: &FumoContext,
        cmd: InteractionCommand,
    ) -> Result<()> {
        let command = Self::from_interaction(cmd.data.clone().into())?;

        match command {
            OsuCommands::Link(command) => {
                ctx.stats.bot.cmd.with_label_values(&["osu_link"]).inc();
                command.run(ctx, cmd).await
            },
            OsuCommands::Unlink(command) => {
                ctx.stats.bot.cmd.with_label_values(&["osu_unlink"]).inc();
                command.run(ctx, cmd).await
            },
            OsuCommands::Attributes(attrs) => match attrs {
                OsuAttributes::Ar(command) => {
                    ctx.stats.bot.cmd.with_label_values(&["osu_attributes_ar"]).inc();
                    command.run(ctx, cmd).await
                },
                OsuAttributes::Od(command) => {
                    ctx.stats.bot.cmd.with_label_values(&["osu_attributes_od"]).inc();
                    command.run(ctx, cmd).await
                },
            },
            OsuCommands::Tracking(command) => match command {
                OsuTracking::Add(command) => {
                    ctx.stats.bot.cmd.with_label_values(&["osu_tracking_ad"]).inc();
                    command.run(ctx, cmd).await
                },
                OsuTracking::Remove(command) => {
                    ctx.stats.bot.cmd.with_label_values(&["osu_tracking_remove"]).inc();
                    command.run(ctx, cmd).await
                },
                OsuTracking::AddBulk(command) => {
                    ctx.stats.bot.cmd.with_label_values(&["osu_tracking_addbulk"]).inc();
                    command.run(ctx, cmd).await
                },
                OsuTracking::RemoveAll(command) => {
                    ctx.stats.bot.cmd.with_label_values(&["osu_tracking_removeall"]).inc();
                    command.run(ctx, cmd).await
                },
                OsuTracking::List(command) => {
                    ctx.stats.bot.cmd.with_label_values(&["osu_tracking_list"]).inc();
                    command.run(ctx, cmd).await
                },
            },
        }
    }
}

/// Unlink an osu! account
#[derive(CommandModel, CreateCommand, Debug)]
#[command(name = "unlink")]
pub struct OsuUnlink {}

impl OsuUnlink {
    pub async fn run(
        &self,
        ctx: &FumoContext,
        cmd: InteractionCommand,
    ) -> Result<()> {
        let osu_user = osu_user!(ctx, cmd);
        let mut msg = MessageBuilder::new().flags(MessageFlags::EPHEMERAL);

        if osu_user.is_none() {
            msg = msg.content("No linked account found!");
            cmd.response(ctx, &msg).await?;
            return Ok(());
        }

        ctx.db.unlink_osu(discord_id!(cmd).get() as i64).await?;

        msg = msg.content("Successfully unlinked account!");

        cmd.response(ctx, &msg).await?;

        Ok(())
    }
}

/// Link an osu! account
#[derive(CommandModel, CreateCommand, Debug)]
#[command(name = "link")]
pub struct OsuLink {
    /// osu! username
    #[command(min_length = 3, max_length = 15)]
    username: String,
}

impl OsuLink {
    pub async fn run(
        &self,
        ctx: &FumoContext,
        cmd: InteractionCommand,
    ) -> Result<()> {
        let osu_user = osu_user!(ctx, cmd);
        let mut msg = MessageBuilder::new().flags(MessageFlags::EPHEMERAL);

        if osu_user.is_some() {
            msg = msg
                .content(r#"You already have linked account. Please use `/unlink` to unlink it."#);

            cmd.response(ctx, &msg).await?;
            return Ok(());
        }

        let user = ctx
            .osu_api
            .get_user(UserId::Username(self.username.to_owned()), None)
            .await?;

        match user {
            Some(user) => {
                ctx.db
                    .link_osu(discord_id!(cmd).get() as i64, user.id)
                    .await?;
            }
            None => {
                msg = msg.content("User not found!");
                cmd.response(ctx, &msg).await?;
                return Ok(());
            }
        };

        msg = msg.content("Successfully linked account!");

        cmd.response(ctx, &msg).await?;

        Ok(())
    }
}
