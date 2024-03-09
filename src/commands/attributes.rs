use crate::{
    fumo_context::FumoContext,
    utils::{ 
        InteractionCommand, ms_to_ar,  hit_windows_circle_std,
        ar_to_ms,
        MessageBuilder,
    },
    osu_api::models::OsuMods,
};

use std::str::FromStr;
use std::fmt::Write;

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

    #[command(min_value=1.0, max_value=10.0)]
    ar: f64,
    
    /// osu! valid mods
    mods: Option<String>,
}

impl OsuAr {
    pub async fn run(
        &self,
        ctx: &FumoContext,
        cmd: InteractionCommand
    ) -> Result<()> {
        // Unwrap cuz ar option is required and there's no way this could fail
        let mut ar = self.ar;
        let mods = if let Some(mods) = &self.mods {
            OsuMods::from_str(mods.as_str()).unwrap()
        } else {
            OsuMods::NOMOD
        };


        let old_ar = ar;

        // Apply EZ 
        if mods.contains(OsuMods::EASY) {
            ar /= 2.0;
        }

        // Apply HR
        if mods.contains(OsuMods::HARDROCK) {
            ar = (ar * 1.4).min(10.0);
        }

        // Calculate ms only after applying EZ & HR
        let mut ms = ar_to_ms(ar);

        if mods.contains(OsuMods::DOUBLETIME) {
            ms /= 1.5;
        }

        if mods.contains(OsuMods::HALFTIME) {
            ms /= 0.75;
        }

        ar = ms_to_ar(ms);

        cmd.defer(ctx).await?;

        let mut msg = MessageBuilder::new();
        msg = msg.content(
            format!(
                "{} -> {:.2} ({:.0}ms) ({})",
                old_ar, ar, ms, mods.to_string()
                )
            );

        cmd.update(ctx, &msg).await?;

        Ok(())

    }
}

/// Calculate OD
#[derive(CommandModel, CreateCommand, Debug)]
#[command(name = "od")]
pub struct OsuOd {
    /// OD of the beatmap
    #[command(min_value=1.0, max_value=10.0)]
    od: f64,
    
    /// osu! valid mods
    mods: Option<String>,
}

impl OsuOd {
    pub async fn run(
        &self,
        ctx: &FumoContext,
        cmd: InteractionCommand
    ) -> Result<()> {
        let mut st = String::new();
        // Unwrap cuz `od` option is required and there's no way this could fail
        let mut od = self.od;
        let mods = if let Some(mods) = &self.mods {
            OsuMods::from_str(mods.as_str()).unwrap()
        } else {
            OsuMods::NOMOD
        };

        if mods.contains(OsuMods::EASY) {
            od /= 2.0;
        }

        if mods.contains(OsuMods::HARDROCK) {
            od = (od * 1.4).min(10.0);
        }

        let (mut c300, mut c100, mut c50) = hit_windows_circle_std(od);

        if mods.contains(OsuMods::DOUBLETIME) {
            c300 /= 1.5;
            c100 /= 1.5;
            c50 /= 1.5;
        }

        if mods.contains(OsuMods::HALFTIME) {
            c300 /= 0.75;
            c100 /= 0.75;
            c50 /= 0.75;
        }

        cmd.defer(ctx).await?;

        let new_od = (c300 - 80.0) / - 6.0;

        let _ = writeln!(st, "```{od} -> {:.2} ({})", new_od, mods.to_string());
        let _  = writeln!(st, "300: ±{c300:.2}ms");
        let _  = writeln!(st, "100: ±{c100:.2}ms");
        let _  = writeln!(st, "50: ±{c50:.2}ms```");

        let mut msg = MessageBuilder::new();
        msg = msg.content(st);
        cmd.update(ctx, &msg).await?;

        Ok(())

    }
}
