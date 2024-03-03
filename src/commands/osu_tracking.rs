use twilight_interactions::command::{CommandModel, CreateCommand};
use crate::{fumo_context::FumoContext, utils::InteractionCommand};
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
        cmd.defer(&ctx).await?;

        println!("{}", self.osu_user);

        Ok(())
    }
}
