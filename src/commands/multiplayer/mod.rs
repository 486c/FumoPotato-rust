use compare::MultiplayerCompare;
use eyre::Result;
use leaderboard::MultiplayerLeaderboard;
use metrics::counter;
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
                counter!("commands", "kind" => "multiplayer_list").increment(1);
                command.run(ctx, cmd).await
            }
            MultiplayerCommands::Compare(command) => {
                counter!("commands", "kind" => "multiplayer_compare").increment(1);
                command.run(ctx, cmd).await
            }
            MultiplayerCommands::Leaderboard(command) => {
                counter!("commands", "kind" => "multiplayer_leaderboard").increment(1);
                command.run(ctx, cmd).await
            }
        }
    }
}
