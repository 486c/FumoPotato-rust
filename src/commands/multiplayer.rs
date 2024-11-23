use eyre::Result;
use twilight_interactions::command::{CommandModel, CommandOption, CreateCommand, CreateOption};
use std::fmt::Write;
use twilight_model::channel::message::MessageFlags;

use crate::{fumo_context::FumoContext, utils::{InteractionCommand, MessageBuilder}};


/// All osu! multiplayer related commands
#[derive(CommandModel, CreateCommand, Debug)]
#[command(name = "multiplayer")]
pub enum MultiplayerCommands {
    #[command(name = "list")]
    List(MultiplayerList)
}

impl MultiplayerCommands {
    pub async fn handle(
        ctx: &FumoContext,
        cmd: InteractionCommand
    ) -> Result<()> {
        let command = Self::from_interaction(
            cmd.data.clone().into()
        )?;

        match command {
            MultiplayerCommands::List(command) => command.run(ctx, cmd).await,
        }
    }
}

#[derive(Debug, CommandOption, CreateOption)]
pub enum ListKind {
    #[option(name = "All", value = "all")]
    All = 0,
    #[option(name = "Tournament", value = "tournament")]
    Tournament = 1,
}

/// List all matches which player participated in
#[derive(CommandModel, CreateCommand, Debug)]
#[command(name = "list")]
pub struct MultiplayerList {
    /// List all matches or only tournament related
    kind: ListKind,
}

impl MultiplayerList {
    pub async fn run(
        &self,
        ctx: &FumoContext,
        cmd: InteractionCommand
    ) -> Result<()> {

        let osu_user = osu_user!(ctx, cmd);

        if osu_user.is_none() {
            let msg = MessageBuilder::new()
                .flags(MessageFlags::EPHEMERAL)
                .content("No linked account found!");
            cmd.response(ctx, &msg).await?;
            return Ok(())
        }

        cmd.defer(ctx).await?;

        let osu_user = osu_user.unwrap();

        let matches = match self.kind {
            ListKind::All => ctx.db.get_user_matches_all(osu_user.osu_id).await?,
            ListKind::Tournament => ctx.db.get_user_matches_tourney(osu_user.osu_id).await?,
        };

        let mut text = String::new();
        let mut msg = MessageBuilder::new();

        let _ = write!(text, "Found {} matches\n", matches.len());
        let _ = writeln!(text, "```");

        for m in matches {
            let _ = writeln!(
                text,
                "{} -> https://osu.ppy.sh/community/matches/{}", 
                m.name, m.id
            );
        }

        let _ = writeln!(text, "```");

        msg = msg.content(text);

        cmd.update(ctx, &msg).await?;

        Ok(())
    }
}
