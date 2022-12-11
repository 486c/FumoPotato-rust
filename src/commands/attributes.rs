use crate::{
    fumo_context::FumoContext,
    utils::{ 
        InteractionCommand, ar_to_ms, ms_to_ar,
        MessageBuilder,
    },
    osu_api::models::OsuMods,
};

use std::str::FromStr;

use eyre::{ Result, bail };


async fn ar(
    ctx: &FumoContext, 
    cmd: InteractionCommand, 
    mods: OsuMods
) -> Result<()> {
    // Unwrap cuz ar option is required and there's no way this could fail
    let mut ar = cmd.get_option_number("ar").unwrap();

    // Apply EZ 
    if mods.contains(OsuMods::EASY) {
        ar /= 2.0;
    }

    // Apply HR
    if mods.contains(OsuMods::HARDROCK) {
        ar = (ar * 1.4).min(10.0);
    }

    // Calculate ms only after applying EZ & HR
    let mut ms: f64 = if ar > 5.0 {
        1200.0 - 750.0 * (ar - 5.0) / 5.0
    } else if ar < 5.0 {
        1200.0 + 600.0 * (5.0 - ar) / 5.0
    } else {
        1200.0
    };

    if mods.contains(OsuMods::DOUBLETIME) {
        ms /= 1.5;
    }

    if mods.contains(OsuMods::HALFTIME) {
        ms /= 0.75;
    }

    ar = ms_to_ar(ms);

    cmd.defer(&ctx).await?;

    let mut msg = MessageBuilder::new();
    msg = msg.content(
        format!("{:.2} ({:.0}ms)", ar, ms)
    );

    cmd.update(&ctx, &msg).await?;

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

    // Should never fail so using unwrap here but this is still really silly
    let mods = OsuMods::from_str(mods_str.as_str()).unwrap();

    match command.data.name.as_str() {
        "ar" => ar(ctx, command, mods).await,
        "od" => Ok(()),
        _ => bail!("Got unexpected command!"),
    }
}
