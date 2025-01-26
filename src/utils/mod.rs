pub mod interaction;
pub mod static_components;

use std::ops;

use once_cell::sync::OnceCell;
use osu_api::models::{osu_mods::OsuModsLazer, OsuGameMode};

// regex utils
pub struct Regex {
    regex: &'static str,
    cell: OnceCell<regex::Regex>,
}

impl Regex {
    const fn new(regex: &'static str) -> Self {
        Self {
            regex,
            cell: OnceCell::new(),
        }
    }

    pub fn get(&self) -> &regex::Regex {
        self.cell
            .get_or_init(|| regex::Regex::new(self.regex).unwrap())
    }
}

macro_rules! define_regex {
    ($($name:ident: $pat:literal;)*) => {
        $( pub static $name: Regex = Regex::new($pat); )*
    };
}

define_regex! {
    OSU_MAP_ID_NEW: r"https://osu.ppy.sh/beatmapsets/(\d+)(?:(?:#(?:osu|mania|taiko|fruits)|<#\d+>)/(\d+))?";
    OSU_MAP_ID_OLD: r"https://osu.ppy.sh/b(?:eatmaps)?/(\d+)";
}

#[macro_export]
macro_rules! random_string {
    ($count:expr) => {
        Alphanumeric.sample_string(&mut rand::thread_rng(), $count)
    };
}

pub enum HitWindow {
    Osu(f64, f64, f64),
    Mania(f64, f64, f64, f64, f64),
    Taiko(f64, f64, f64),
    Fruits,
}

impl HitWindow {
    pub fn to_od(&self) -> f64 {
        match self {
            HitWindow::Osu(c300, _, _) => (c300 - 80.0) / -6.0,
            HitWindow::Mania(_, c300, _, _, _) => (c300 - 64.0) / -3.0,
            HitWindow::Taiko(great, _, _) => (great - 50.0) / -3.0,
            HitWindow::Fruits => 0.0,
        }
    }
}

impl ops::Div<f64> for HitWindow {
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        match self {
            HitWindow::Osu(c300, c100, c50) => Self::Osu(c300 / rhs, c100 / rhs, c50 / rhs),
            HitWindow::Mania(max, c300, c200, c100, c50) => Self::Mania(
                max / rhs,
                c300 / rhs,
                c200 / rhs,
                c100 / rhs,
                c50 / rhs
            ),
            HitWindow::Taiko(great, ok, miss) => Self::Taiko(great / rhs, ok / rhs, miss / rhs),
            HitWindow::Fruits => Self::Fruits,
        }
    }
}

#[inline]
pub fn hit_windows_circle_std(od: f64) -> (f64, f64, f64) {
    (80.0 - 6.0 * od, 140.0 - 8.0 * od, 200.0 - 10.0 * od)
}


// TODO to new function impl
#[inline]
pub fn hit_window(od: f64, mode: &OsuGameMode) -> HitWindow {
    match mode {
        OsuGameMode::Fruits => HitWindow::Fruits,
        OsuGameMode::Mania => HitWindow::Mania(
            16.0, 
            64.0 - 3.0 * od,
            97.0 - 3.0 * od,
            127.0 - 3.0 * od,
            151.0 - 3.0 * od 
        ),
        OsuGameMode::Osu => HitWindow::Osu(80.0 - 6.0 * od, 140.0 - 8.0 * od, 200.0 - 10.0 * od),
        OsuGameMode::Taiko => {
            let great = 50.0 - 3.0 * od;

            let (ok, miss) = if od <= 5.0 {
                (
                    120.0 - 8.0 * od,
                    135.0 - 8.0 * od 
                )
            } else {
                (
                    110.0 - 6.0 * od,
                    120.0 - 5.0 * od
                )
            };

            HitWindow::Taiko(great, ok, miss)
        },
    }
}

#[inline]
pub fn ar_to_ms(ar: f64) -> f64 {
    if ar > 5.0 {
        1200.0 - 750.0 * (ar - 5.0) / 5.0
    } else if ar < 5.0 {
        1200.0 + 600.0 * (5.0 - ar) / 5.0
    } else {
        1200.0
    }
}

#[inline]
pub fn ms_to_ar(ms: f64) -> f64 {
    if ms < 1200.0 {
        ((ms * 5.0 - 1200.0 * 5.0) / (450.0 - 1200.0)) + 5.0
    } else if ms > 1200.0 {
        5.0 - ((1200.0 * 5.0 - ms * 5.0) / (1200.0 - 1800.0))
    } else {
        1200.0
    }
}


pub fn calc_ar(ar: f32, mods: &OsuModsLazer) -> f64 {
    let mut ar = ar as f64;

    if mods.contains("EZ") {
        ar /= 2.0;
    }

    if mods.contains("HR") {
        ar = (ar * 1.4).min(10.0);
    }

    let mut ms = ar_to_ms(ar);

    if let Some(speed_change) = mods.speed_changes() {
        ms /= speed_change as f64;

        return ms_to_ar(ms);
    }

    if mods.contains("DT") {
        ms /= 1.5;
    }

    if mods.contains("HT") {
        ms /= 0.75;
    }

    ms_to_ar(ms)
}

pub fn calc_od(od: f32, mods: &OsuModsLazer, mode: &OsuGameMode) -> f64 {
    let mut od = od as f64;

    match mode {
        OsuGameMode::Fruits => return od,
        _ => {}
    };

    if mods.contains("EZ") {
        od /= 2.0;
    }

    if mods.contains("HR") {
        od = (od * 1.4).min(10.0);
    }

    let mut hit_window = hit_window(od, mode);

    if let Some(speed_change) = mods.speed_changes() {
        hit_window = hit_window / speed_change as f64;

        return hit_window.to_od()
    }

    if mods.contains("DT") {
        hit_window = hit_window / 1.5;
    }

    if mods.contains("HT") {
        hit_window = hit_window / 0.75;
    }

    hit_window.to_od()
}
