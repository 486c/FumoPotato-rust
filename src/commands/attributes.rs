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

use eyre::{ Result, bail };

async fn ar(
    ctx: &FumoContext, 
    cmd: InteractionCommand, 
    mods: OsuMods
) -> Result<()> {
    // Unwrap cuz ar option is required and there's no way this could fail
    let mut ar = cmd.get_option_number("ar").unwrap();

    let old_ar = ar.clone();

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

async fn od(
    ctx: &FumoContext, 
    cmd: InteractionCommand, 
    mods: OsuMods
) -> Result<()> {
    let mut st = String::new();
    // Unwrap cuz `od` option is required and there's no way this could fail
    let mut od = cmd.get_option_number("od").unwrap();

    let _ = writeln!(st, "```{od} +{}", mods.to_string());

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

    let _  = writeln!(st, "300: {c300:.2}ms");
    let _  = writeln!(st, "100: {c100:.2}ms");
    let _  = writeln!(st, "50: {c50:.2}ms```");

    let mut msg = MessageBuilder::new();
    msg = msg.content(st);
    cmd.update(ctx, &msg).await?;

    Ok(())
}

pub async fn run(ctx: &FumoContext, command: InteractionCommand) -> Result<()> {
    // Getting mods
    let mods_str = if let Some(mods) = command.get_option_string("mods") {
        mods.chars()
            .filter(|c| *c != '+')
            .filter(|c| *c != '-')
        .collect()
    } else {
        String::default()
    };

    // Using unwrap cuz it can't panic in any way:
    // Cuz even if `OsuMods` fails to parse, it still will 
    // return empty struct that equals to NM
    let mods = OsuMods::from_str(mods_str.as_str())
        .unwrap();

    match command.data.name.as_str() {
        "ar" => ar(ctx, command, mods).await,
        "od" => od(ctx, command, mods).await,
        _ => bail!("Got unexpected command!"),
    }
}
