use crate::{
    fumo_context::FumoContext,
    utils::{
        ar_to_ms, calc_ar, calc_od, hit_window, hit_windows_circle_std,
        interaction::InteractionCommand, ms_to_ar, HitWindow,
    },
};

use fumo_twilight::message::MessageBuilder;
use osu_api::models::{osu_mods::OsuModsLazer, OsuGameMode, OsuMods};

use std::{fmt::Write, str::FromStr};

use eyre::Result;
use twilight_interactions::command::{CommandModel, CreateCommand};

/// osu! attributes stuff
#[derive(CommandModel, CreateCommand, Debug)]
#[command(name = "attributes")]
pub enum OsuAttributes {
    #[command(name = "ar")]
    Ar(OsuAr),
    #[command(name = "od")]
    Od(OsuOd),
}

/// Calculate AR
#[derive(CommandModel, CreateCommand, Debug)]
#[command(name = "ar")]
pub struct OsuAr {
    /// AR of the beatmap
    #[command(min_value = 1.0, max_value = 10.0)]
    ar: f64,

    /// osu! valid mods
    mods: Option<String>,
}

impl OsuAr {
    pub async fn run(
        &self,
        ctx: &FumoContext,
        cmd: InteractionCommand,
    ) -> Result<()> {
        // Unwrap cuz ar option is required and there's no way this could fail
        let ar = self.ar;

        let mods = if let Some(mods) = &self.mods {
            OsuModsLazer::from_str(mods.as_str()).unwrap()
        } else {
            OsuModsLazer::default()
        };

        let old_ar = ar;

        let ar = calc_ar(old_ar as f32, &mods);
        let ms = ar_to_ms(ar);

        cmd.defer(ctx).await?;

        let mut msg = MessageBuilder::new();
        msg = msg.content(format!(
            "{} -> {:.2} ({:.0}ms) ({})",
            old_ar, ar, ms, mods
        ));

        cmd.update(ctx, &msg).await?;

        Ok(())
    }
}

/// Calculate OD
#[derive(CommandModel, CreateCommand, Debug)]
#[command(name = "od")]
pub struct OsuOd {
    /// OD of the beatmap
    #[command(min_value = 1.0, max_value = 10.0)]
    od: f64,

    /// osu! valid mods
    mods: Option<String>,
}

impl OsuOd {
    pub async fn run(
        &self,
        ctx: &FumoContext,
        cmd: InteractionCommand,
    ) -> Result<()> {
        let mut st = String::new();
        // Unwrap cuz `od` option is required and there's no way this could fail
        let od = self.od;

        let mods = if let Some(mods) = &self.mods {
            OsuModsLazer::from_str(mods.as_str()).unwrap()
        } else {
            OsuModsLazer::default()
        };

        let new_od = calc_od(od as f32, &mods, &OsuGameMode::Osu);
        let HitWindow::Osu(c300, c100, c50) =
            hit_window(new_od, &OsuGameMode::Osu)
        else {
            cmd.defer(ctx).await?;
            let msg = MessageBuilder::new()
                .content("Internal error during calculation. blame lopij");
            cmd.update(ctx, &msg).await?;
            return Ok(());
        };

        cmd.defer(ctx).await?;

        let _ = writeln!(st, "```{od} -> {:.2} ({})", new_od, mods);
        let _ = writeln!(st, "300: ±{c300:.2}ms");
        let _ = writeln!(st, "100: ±{c100:.2}ms");
        let _ = writeln!(st, "50: ±{c50:.2}ms```");

        let mut msg = MessageBuilder::new();
        msg = msg.content(st);
        cmd.update(ctx, &msg).await?;

        Ok(())
    }
}
