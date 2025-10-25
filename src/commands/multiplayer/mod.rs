use compare::MultiplayerCompare;
use eyre::Result;
use leaderboard::MultiplayerLeaderboard;
use twilight_interactions::command::{
    CommandModel, CommandOption, CreateCommand, CreateOption,
};

use crate::{
    fumo_context::FumoContext, utils::interaction::InteractionCommand,
};

mod compare;
mod leaderboard;
mod list;

use list::MultiplayerList;

#[derive(Debug, CommandOption, CreateOption)]
pub enum ListKind {
    #[option(name = "All", value = "all")]
    All = 0,
    #[option(name = "Tournament", value = "tournament")]
    Tournament = 1,
}

impl ListKind {
    fn is_tournament(&self) -> bool {
        match self {
            ListKind::All => false,
            ListKind::Tournament => true,
        }
    }
}

/// All osu! multiplayer related commands
#[derive(CommandModel, CreateCommand, Debug)]
#[command(name = "multiplayer")]
pub enum MultiplayerCommands {
    #[command(name = "list")]
    List(MultiplayerList),
    #[command(name = "compare")]
    Compare(MultiplayerCompare),
    #[command(name = "leaderboard")]
    Leaderboard(MultiplayerLeaderboard),
}

impl MultiplayerCommands {
    pub async fn handle(
        ctx: &FumoContext,
        cmd: InteractionCommand,
    ) -> Result<()> {
        let command = Self::from_interaction(cmd.data.clone().into())?;

        match command {
            MultiplayerCommands::List(command) => {
                ctx.stats
                    .bot
                    .cmd
                    .with_label_values(&["multiplayer_list"])
                    .inc();
                command.run(ctx, cmd).await
            }
            MultiplayerCommands::Compare(command) => {
                ctx.stats
                    .bot
                    .cmd
                    .with_label_values(&["multiplayer_compare"])
                    .inc();
                command.run(ctx, cmd).await
            }
            MultiplayerCommands::Leaderboard(command) => {
                ctx.stats
                    .bot
                    .cmd
                    .with_label_values(&["multiplayer_leaderboard"])
                    .inc();
                command.run(ctx, cmd).await
            }
        }
    }
}
