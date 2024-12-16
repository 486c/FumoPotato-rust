pub mod interaction;
pub mod static_components;

use once_cell::sync::OnceCell;

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

#[inline]
pub fn hit_windows_circle_std(od: f64) -> (f64, f64, f64) {
    (80.0 - 6.0 * od, 140.0 - 8.0 * od, 200.0 - 10.0 * od)
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

