pub mod osu_leaderboard;
pub mod osu_matches;
pub mod osu_mods;

use crate::{datetime, models::osu_mods::OsuModsLazer};

use crate::error::OsuApiError;

use chrono::prelude::*;

use osu_leaderboard::OsuScoreLazer;
use serde::{
    de::{self, Deserializer, Error, SeqAccess, Unexpected, Visitor},
    Deserialize,
};
use thiserror::Error;

use std::ops;
use std::{
    fmt::{self, Display},
    str::FromStr,
};

use bitflags::bitflags;

#[derive(Error, Debug, Deserialize)]
pub struct ApiError {
    pub error: Option<String>,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.error {
            Some(s) => f.write_str(s),
            None => f.write_str("empty error message"),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(i8)]
pub enum RankStatus {
    Graveyard = -2,
    Wip = -1,
    Pending = 0,
    Ranked = 1,
    Approved = 2,
    Qualified = 3,
    Loved = 4,
}

impl Display for RankStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RankStatus::Graveyard => write!(f, "Graveyard"),
            RankStatus::Wip => write!(f, "Wip"),
            RankStatus::Pending => write!(f, "Pending"),
            RankStatus::Ranked => write!(f, "Ranked"),
            RankStatus::Approved => write!(f, "Approved"),
            RankStatus::Qualified => write!(f, "Qualified"),
            RankStatus::Loved => write!(f, "Loved"),
        }
    }
}

struct RankStatusVisitor;

impl Visitor<'_> for RankStatusVisitor {
    type Value = RankStatus;

    #[inline]
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a valid rank status integer")
    }

    fn visit_u64<E: Error>(self, v: u64) -> Result<Self::Value, E> {
        match v {
            0 => Ok(RankStatus::Pending),
            1 => Ok(RankStatus::Ranked),
            2 => Ok(RankStatus::Approved),
            3 => Ok(RankStatus::Qualified),
            4 => Ok(RankStatus::Loved),
            _ => Err(Error::invalid_value(
                Unexpected::Unsigned(v),
                &r#"0, 1, 2, 3 or 4"#,
            )),
        }
    }

    fn visit_i64<E: Error>(self, v: i64) -> Result<Self::Value, E> {
        match v {
            -2 => Ok(RankStatus::Graveyard),
            -1 => Ok(RankStatus::Wip),
            0 => Ok(RankStatus::Pending),
            1 => Ok(RankStatus::Ranked),
            2 => Ok(RankStatus::Approved),
            3 => Ok(RankStatus::Qualified),
            4 => Ok(RankStatus::Loved),
            _ => Err(Error::invalid_value(
                Unexpected::Signed(v),
                &r#"-2, -1, 0, 1, 2, 3 or 4"#,
            )),
        }
    }

    fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
        match v {
            "graveyard" => Ok(RankStatus::Graveyard),
            "wip" => Ok(RankStatus::Wip),
            "pending" => Ok(RankStatus::Pending),
            "ranked" => Ok(RankStatus::Ranked),
            "approved" => Ok(RankStatus::Approved),
            "qualified" => Ok(RankStatus::Qualified),
            "loved" => Ok(RankStatus::Loved),
            _ => Err(Error::invalid_value(
                Unexpected::Str(v),
                &r#"ranked, graveyard, wip and other"#,
            )),
        }
    }
}

impl<'de> Deserialize<'de> for RankStatus {
    #[inline]
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_any(RankStatusVisitor)
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum OsuGrade {
    GradeXH,
    GradeSH,
    GradeX,
    GradeS,
    GradeA,
    GradeB,
    GradeC,
    GradeD,
    GradeF,
}

impl OsuGrade {
    pub fn to_emoji(&self) -> &str {
        match self {
            OsuGrade::GradeXH => "<:r_XH:1004444329365999766>",
            OsuGrade::GradeSH => "<:r_SH:1004444326669066270>",
            OsuGrade::GradeX => "<:r_X:1004444328082538546>",
            OsuGrade::GradeS => "<:r_S:1004444324840349759>",
            OsuGrade::GradeA => "<:r_A:1004444322365702204>",
            OsuGrade::GradeB => "<:r_B:1004444032149233696>",
            OsuGrade::GradeC => "<:r_C:1004444033524957235>",
            OsuGrade::GradeD => "<:r_D:1004444323703701545>",
            OsuGrade::GradeF => "<:r_D:1004444323703701545>",
        }
    }
}

impl fmt::Display for OsuGrade {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OsuGrade::GradeXH => write!(f, "XH"),
            OsuGrade::GradeSH => write!(f, "SH"),
            OsuGrade::GradeX => write!(f, "X"),
            OsuGrade::GradeS => write!(f, "S"),
            OsuGrade::GradeA => write!(f, "A"),
            OsuGrade::GradeB => write!(f, "B"),
            OsuGrade::GradeC => write!(f, "C"),
            OsuGrade::GradeD => write!(f, "D"),
            OsuGrade::GradeF => write!(f, "F"),
        }
    }
}

struct OsuRankVisitor;

impl Visitor<'_> for OsuRankVisitor {
    type Value = OsuGrade;

    #[inline]
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a valid rank string")
    }

    fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
        let rank = match v {
            "XH" => OsuGrade::GradeXH,
            "SH" => OsuGrade::GradeSH,
            "X" => OsuGrade::GradeX,
            "S" => OsuGrade::GradeS,
            "A" => OsuGrade::GradeA,
            "B" => OsuGrade::GradeB,
            "C" => OsuGrade::GradeC,
            "D" => OsuGrade::GradeD,
            "F" => OsuGrade::GradeF,
            _ => {
                return Err(Error::invalid_value(
                    Unexpected::Str(v),
                    &r#""XH", "SH", "X", "S", "A", "B", "C", "D" or "F""#,
                ))
            }
        };

        Ok(rank)
    }
}

impl<'de> Deserialize<'de> for OsuGrade {
    #[inline]
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_any(OsuRankVisitor)
    }
}

bitflags! {
    #[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
    pub struct OsuMods: u32 {
        const NOMOD = 0;
        const NOFAIL = u32::pow(2, 0);
        const EASY = u32::pow(2, 1);
        const TOUCHDEVICE = u32::pow(2, 2);
        const HIDDEN = u32::pow(2, 3);
        const HARDROCK = u32::pow(2, 4);
        const SUDDENDEATH = u32::pow(2, 5);
        const DOUBLETIME = u32::pow(2, 6);
        const RELAX = u32::pow(2, 7);
        const HALFTIME = u32::pow(2, 8);
        const NIGHTCORE = u32::pow(2, 9) | Self::DOUBLETIME.bits();
        const FLASHLIGHT = u32::pow(2, 10);
        const AUTOPLAY = u32::pow(2, 11); //
        const SPUNOUT = u32::pow(2, 12);
        const AUTOPILOT = u32::pow(2, 13);
        const PERFECT = u32::pow(2, 14) | Self::SUDDENDEATH.bits();
        const KEY4 = u32::pow(2, 15);
        const KEY5 = u32::pow(2, 16);
        const KEY6 = u32::pow(2, 17);
        const KEY7 = u32::pow(2, 18);
        const KEY8 = u32::pow(2, 19);
        const FADEIN = u32::pow(2, 20);
        const RANDOM = u32::pow(2, 21);
        const CINEMA = u32::pow(2, 22); //
        const TARGET = u32::pow(2, 23); //
        const KEY9 = u32::pow(2, 24);
        const KEYCOOP = u32::pow(2, 25);
        const KEY1 = u32::pow(2, 26);
        const KEY3 = u32::pow(2, 27);
        const KEY2 = u32::pow(2, 28);
        const SCOREV2 = u32::pow(2, 29);
        const MIRROR = u32::pow(2, 30);
    }
}

impl OsuMods {
    fn from_acronym_str(abbrev: &str) -> Option<OsuMods> {
        match abbrev {
            "NM" => Some(OsuMods::NOMOD),
            "NF" => Some(OsuMods::NOFAIL),
            "EZ" => Some(OsuMods::EASY),
            "TD" => Some(OsuMods::TOUCHDEVICE),
            "HD" => Some(OsuMods::HIDDEN),
            "HR" => Some(OsuMods::HARDROCK),
            "SD" => Some(OsuMods::SUDDENDEATH),
            "DT" => Some(OsuMods::DOUBLETIME),
            "RX" => Some(OsuMods::RELAX),
            "HT" => Some(OsuMods::HALFTIME),
            "NC" => Some(OsuMods::NIGHTCORE),
            "FL" => Some(OsuMods::FLASHLIGHT),
            "SO" => Some(OsuMods::SPUNOUT),
            "PF" => Some(OsuMods::PERFECT),
            "FI" => Some(OsuMods::FADEIN),
            "MR" => Some(OsuMods::MIRROR),
            "AP" => Some(OsuMods::AUTOPILOT),
            "1K" => Some(OsuMods::KEY1),
            "2K" => Some(OsuMods::KEY2),
            "3K" => Some(OsuMods::KEY3),
            "4K" => Some(OsuMods::KEY4),
            "5K" => Some(OsuMods::KEY5),
            "6K" => Some(OsuMods::KEY6),
            "7K" => Some(OsuMods::KEY7),
            "8K" => Some(OsuMods::KEY8),
            "9K" => Some(OsuMods::KEY9),
            "RD" => Some(OsuMods::RANDOM),
            "2P" => Some(OsuMods::KEYCOOP),
            _ => None,
        }
    }
}

impl Display for OsuMods {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_empty() {
            write!(f, "NM")?;
            return Ok(());
        }

        if self.contains(OsuMods::NOFAIL) {
            write!(f, "NF")?;
        }
        if self.contains(OsuMods::EASY) {
            write!(f, "EZ")?;
        }
        if self.contains(OsuMods::TOUCHDEVICE) {
            write!(f, "TD")?;
        }
        if self.contains(OsuMods::HIDDEN) {
            write!(f, "HD")?;
        }
        if self.contains(OsuMods::DOUBLETIME) {
            if self.contains(OsuMods::NIGHTCORE) {
                write!(f, "NC")?;
            } else {
                write!(f, "DT")?;
            }
        }
        if self.contains(OsuMods::HALFTIME) {
            write!(f, "HT")?;
        }
        if self.contains(OsuMods::FLASHLIGHT) {
            write!(f, "FL")?;
        }
        if self.contains(OsuMods::HARDROCK) {
            write!(f, "HR")?;
        }
        if self.contains(OsuMods::SUDDENDEATH) {
            write!(f, "SD")?;
        }
        if self.contains(OsuMods::SPUNOUT) {
            write!(f, "SO")?;
        }
        if self.contains(OsuMods::PERFECT) {
            write!(f, "PF")?;
        }
        if self.contains(OsuMods::MIRROR) {
            write!(f, "MR")?;
        }
        if self.contains(OsuMods::AUTOPILOT) {
            write!(f, "AP")?;
        }
        if self.contains(OsuMods::KEY1) {
            write!(f, "K1")?;
        }
        if self.contains(OsuMods::KEY2) {
            write!(f, "K2")?;
        }
        if self.contains(OsuMods::KEY3) {
            write!(f, "K3")?;
        }
        if self.contains(OsuMods::KEY4) {
            write!(f, "K4")?;
        }
        if self.contains(OsuMods::KEY5) {
            write!(f, "K5")?;
        }
        if self.contains(OsuMods::KEY6) {
            write!(f, "K6")?;
        }
        if self.contains(OsuMods::KEY7) {
            write!(f, "K7")?;
        }
        if self.contains(OsuMods::KEY8) {
            write!(f, "K8")?;
        }
        if self.contains(OsuMods::KEY9) {
            write!(f, "K9")?;
        }
        if self.contains(OsuMods::RANDOM) {
            write!(f, "RD")?;
        }
        if self.contains(OsuMods::KEYCOOP) {
            write!(f, "2P")?;
        }

        Ok(())
    }
}

impl FromStr for OsuMods {
    type Err = OsuApiError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_uppercase();
        let mut flags = OsuMods::empty();

        for abbrev in utils::cut(&s, 2) {
            let mods = Self::from_acronym_str(abbrev);
            if let Some(m) = mods {
                flags |= m
            }
        }

        Ok(flags)
    }
}

struct OsuModsVisitor;

impl<'de> Visitor<'de> for OsuModsVisitor {
    type Value = OsuMods;

    #[inline]
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("valid sequence, string with valid acronyms")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut mods = OsuMods::default();

        while let Some(next) = seq.next_element()? {
            mods |= next;
        }

        Ok(mods)
    }

    fn visit_i64<E: Error>(self, v: i64) -> Result<Self::Value, E> {
        Ok(OsuMods::from_bits_truncate(v as u32))
    }

    fn visit_u64<E: Error>(self, v: u64) -> Result<Self::Value, E> {
        Ok(OsuMods::from_bits_truncate(v as u32))
    }

    fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
        let mods = match OsuMods::from_acronym_str(v) {
            Some(m) => m,
            None => {
                return Err(Error::invalid_value(
                    Unexpected::Str(v),
                    &r#"valid mods acronym"#,
                ))
            }
        };

        Ok(mods)
    }
}

impl<'de> Deserialize<'de> for OsuMods {
    #[inline]
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_any(OsuModsVisitor)
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
#[repr(u8)]
pub enum OsuGameMode {
    Fruits,
    Mania,
    Osu,
    Taiko,
}

impl From<OsuGameMode> for rosu_mods::GameMode {
    fn from(val: OsuGameMode) -> Self {
        match val {
            OsuGameMode::Fruits => rosu_mods::GameMode::Catch,
            OsuGameMode::Mania => rosu_mods::GameMode::Mania,
            OsuGameMode::Osu => rosu_mods::GameMode::Osu,
            OsuGameMode::Taiko => rosu_mods::GameMode::Taiko,
        }
    }
}

impl fmt::Display for OsuGameMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OsuGameMode::Fruits => write!(f, "fruits"),
            OsuGameMode::Mania => write!(f, "mania"),
            OsuGameMode::Osu => write!(f, "osu"),
            OsuGameMode::Taiko => write!(f, "taiko"),
        }
    }
}

impl OsuGameMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            OsuGameMode::Fruits => "fruits",
            OsuGameMode::Mania => "mania",
            OsuGameMode::Osu => "osu",
            OsuGameMode::Taiko => "taiko",
        }
    }

    pub fn as_u8(&self) -> u8 {
        match self {
            OsuGameMode::Osu => 0,
            OsuGameMode::Taiko => 1,
            OsuGameMode::Fruits => 2,
            OsuGameMode::Mania => 3,
        }
    }

    pub fn to_emoji(&self) -> &'static str {
        match self {
            OsuGameMode::Fruits => "<:mode_fruits:1332838034839699549>",
            OsuGameMode::Mania => "<:mode_mania:1332838036676808848>",
            OsuGameMode::Osu => "<:r_mode_osu:1332842409557491745>",
            OsuGameMode::Taiko => "<:mode_taiko:1332838039428403230>",
        }
    }
}

struct OsuGameModeVisitor;

impl Visitor<'_> for OsuGameModeVisitor {
    type Value = OsuGameMode;

    #[inline]
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("valid string")
    }

    fn visit_u64<E: Error>(self, v: u64) -> Result<Self::Value, E> {
        match v {
            0 => Ok(OsuGameMode::Osu),
            1 => Ok(OsuGameMode::Taiko),
            2 => Ok(OsuGameMode::Fruits),
            3 => Ok(OsuGameMode::Mania),
            _ => Err(Error::invalid_value(Unexpected::Unsigned(v), &"0, 1, 2, 3")),
        }
    }

    fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
        match v {
            "osu" => Ok(OsuGameMode::Osu),
            "fruits" => Ok(OsuGameMode::Fruits),
            "taiko" => Ok(OsuGameMode::Taiko),
            "mania" => Ok(OsuGameMode::Mania),
            _ => Err(Error::invalid_value(
                Unexpected::Str(v),
                &"osu, fruits, taiko or mania",
            )),
        }
    }
}

impl<'de> Deserialize<'de> for OsuGameMode {
    #[inline]
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_any(OsuGameModeVisitor)
    }
}

impl TryFrom<&str> for OsuGameMode {
    type Error = OsuApiError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "osu" => Ok(OsuGameMode::Osu),
            "fruits" => Ok(OsuGameMode::Fruits),
            "taiko" => Ok(OsuGameMode::Taiko),
            "mania" => Ok(OsuGameMode::Mania),
            _ => {
                Err(OsuApiError::Casting) // TODO: Think about a better result not this
            }
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct OauthResponse {
    pub token_type: String,
    pub expires_in: i32,
    pub access_token: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct OsuScoreStatistics {
    #[serde(rename = "count_50")]
    pub count50: Option<i32>,
    #[serde(rename = "count_100")]
    pub count100: Option<i32>,
    #[serde(rename = "count_300")]
    pub count300: Option<i32>,
    #[serde(rename = "count_geki")]
    pub countgeki: Option<i32>,
    #[serde(rename = "count_katu")]
    pub countkatu: Option<i32>,
    #[serde(rename = "count_miss")]
    pub countmiss: Option<i32>,
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct OsuBeatmapsetCompact {
    pub title: String,
    pub artist: String,
    pub creator: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct OsuBeatmapAttributes {
    pub attributes: OsuBeatmapAttributesContainer,
}

#[derive(Deserialize, Clone, Debug)]
pub struct OsuBeatmapAttributesContainer {
    pub max_combo: u32,
    pub star_rating: f32,
}

// TODO look at it after rebalance
// #[derive(Deserialize, Clone, Debug)]
// #[serde(untagged)]
// pub enum OsuBeatmapAttributesKind {
// Osu {
// aim_difficulty: f32,
// approach_rate: f32,
// flashlight_difficulty: f32,
// overall_difficulty: f32,
// slider_factor: f32,
// speed_difficulty: f32,
// },
// Taiko {
// stamina_difficulty: f32,
// rhythm_difficulty: f32,
// colour_difficulty: f32,
// approach_rate: f32,
// great_hit_window: f32,
// },
// Fruits {
// approach_rate: f32,
// },
// Mania {
// great_hit_window: f32,
// score_multiplier: Option<f32>,
// },
// }

#[derive(Deserialize, Clone, Debug)]
pub struct OsuBeatmap {
    pub beatmapset_id: i32,
    pub id: i32,
    pub mode: String,
    pub bpm: Option<f32>,

    pub ar: Option<f32>,
    pub cs: Option<f32>,
    pub drain: Option<f32>,
    pub accuracy: Option<f32>,

    pub version: String,

    pub beatmapset: OsuBeatmapsetCompact,

    pub max_combo: Option<i32>,
    pub status: RankStatus,
}

impl OsuBeatmap {
    // Returns `{Artist} - {Title} [{Version}]`
    pub fn metadata(&self) -> String {
        format!(
            "{} \\- {} [{}]",
            self.beatmapset.artist, self.beatmapset.title, self.version
        )
    }

    pub fn attributes(
        &self,
        mods: &OsuModsLazer,
        gamemode: OsuGameMode,
    ) -> Option<BeatmapAttributes> {
        let b_ar = self.ar?;
        let b_cs = self.cs?;
        let b_hp = self.drain?;
        let b_od = self.accuracy?;

        let mut attributes = BeatmapAttributes {
            mode: gamemode,
            ar: b_ar,
            cs: b_cs,
            od: b_od,
            hp: b_hp,
        };

        attributes.calc_ar(mods);
        attributes.calc_od(mods);
        attributes.calc_hp(mods);
        attributes.calc_cs(mods);

        Some(attributes)
    }
}

pub struct BeatmapAttributes {
    mode: OsuGameMode,

    pub ar: f32,
    pub cs: f32,
    pub od: f32,
    pub hp: f32,
}

impl BeatmapAttributes {
    pub fn calc_ar(&mut self, mods: &OsuModsLazer) {
        let mut ar = self.ar;
        if mods.contains("EZ") {
            ar /= 2.0;
        }

        if mods.contains("HR") {
            ar = (ar * 1.4).min(10.0);
        }

        let mut ms = ar_to_ms(ar);

        if let Some(speed_change) = mods.speed_changes() {
            ms /= speed_change;

            self.ar = ms_to_ar(ms);
        }

        if mods.contains("DT") || mods.contains("NC") {
            ms /= 1.5;
        }

        if mods.contains("HT") {
            ms /= 0.75;
        }

        self.ar = ms_to_ar(ms);
    }

    pub fn calc_od(&mut self, mods: &OsuModsLazer) {
        let mut od = self.od as f64;

        if self.mode == OsuGameMode::Fruits {
            return;
        }

        if mods.contains("EZ") {
            od /= 2.0;
        }

        if mods.contains("HR") {
            od = (od * 1.4).min(10.0);
        }

        let mut hit_window = hit_window(od, &self.mode);

        if let Some(speed_change) = mods.speed_changes() {
            hit_window = hit_window / speed_change as f64;

            self.od = hit_window.to_od() as f32;
        }

        if mods.contains("DT") || mods.contains("NC") {
            hit_window = hit_window / 1.5;
        }

        if mods.contains("HT") {
            hit_window = hit_window / 0.75;
        }

        self.od = hit_window.to_od() as f32
    }

    pub fn calc_cs(&mut self, mods: &OsuModsLazer) {
        let mut cs = self.cs;

        if mods.contains("HR") {
            cs = (cs * 1.3).min(10.0);
        };

        if mods.contains("EZ") {
            cs /= 2.0;
        };

        self.cs = cs;
    }

    pub fn calc_hp(&mut self, mods: &OsuModsLazer) {
        let mut hp = self.hp;

        if mods.contains("EZ") {
            hp /= 2.0;
        };

        if mods.contains("HR") {
            hp = (hp * 1.4).min(10.0);
        };

        self.hp = hp;
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct OsuBeatmapScore {
    pub beatmapset_id: i32,
    pub id: i32,
    pub mode: String,

    pub version: String,

    pub max_combo: Option<i32>,
    pub ranked: RankStatus,
}

#[derive(Deserialize, Clone, Debug)]
pub struct OsuBeatmapSetScore {
    pub artist: String,
    pub title: String,
    pub creator: String,
}

#[derive(Deserialize, Clone, Copy, Debug)]
#[repr(u8)]
pub enum OsuScoreMatchTeam {
    None = 0,
    Red = 1,
    Blue = 2,
}

// impl<'r> Decode<'r, Postgres> for OsuScoreMatchTeam {
// fn decode(value: <Postgres as sqlx::Database>::ValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
// let value = <i16 as Decode<'r, Postgres>>::decode(value)?;
//
// Self::try_from(value)
// .map_err(|_| format!("invalid value {value} for OsuScoreMatchTeam").into())
// }
// }
//
// impl Type<Postgres> for OsuScoreMatchTeam {
// #[inline]
// fn type_info() -> PgTypeInfo {
// PgTypeInfo::with_name("INT2")
// }
// }
//
// impl TryFrom<i16> for OsuScoreMatchTeam {
// type Error = OsuApiError;
//
// fn try_from(value: i16) -> Result<Self, OsuApiError> {
// match value {
// 0 => Ok(Self::None),
// 1 => Ok(Self::Red),
// 2 => Ok(Self::Blue),
// _ => Err(OsuApiError::Casting)
// }
// }
//
// }

impl From<i16> for OsuScoreMatchTeam {
    fn from(value: i16) -> Self {
        match value {
            0 => Self::None,
            1 => Self::Red,
            2 => Self::Blue,
            _ => Self::None,
        }
    }
}

struct OsuScoreMatchTeamVisitor;

impl de::Visitor<'_> for OsuScoreMatchTeamVisitor {
    type Value = OsuScoreMatchTeam;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a none, blue, red string")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match value {
            "none" => Ok(OsuScoreMatchTeam::None),
            "red" => Ok(OsuScoreMatchTeam::Red),
            "blue" => Ok(OsuScoreMatchTeam::Blue),
            _ => Err(E::custom(format!(
                "Failed to parse {} as match team",
                value
            ))),
        }
    }
}

pub fn deserialize_match_team<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<OsuScoreMatchTeam, D::Error> {
    d.deserialize_str(OsuScoreMatchTeamVisitor)
}

#[derive(Deserialize, Clone, Debug)]
pub struct OsuScoreMatch {
    pub slot: u8,
    #[serde(deserialize_with = "deserialize_match_team")]
    pub team: OsuScoreMatchTeam,
    #[serde(deserialize_with = "datetime::deserialize_bool::deserialize")]
    pub pass: bool,
}

#[derive(Deserialize, Clone, Debug)]
pub struct OsuScore {
    pub id: Option<i64>,
    pub best_id: Option<i64>,
    pub user_id: i64,
    pub accuracy: f32,

    pub mods: OsuMods,
    pub score: i64,
    pub legacy_total_score: Option<i64>,
    pub classic_total_score: Option<i64>,
    #[serde(deserialize_with = "datetime::deserialize_bool::deserialize")]
    pub perfect: bool,
    #[serde(deserialize_with = "datetime::deserialize_bool::deserialize")]
    pub passed: bool,
    pub pp: Option<f32>,

    pub max_combo: Option<i32>,

    pub rank: OsuGrade,

    #[serde(deserialize_with = "datetime::deserialize::deserialize")]
    pub created_at: DateTime<Utc>,

    #[serde(rename = "statistics")]
    pub stats: OsuScoreStatistics,

    pub mode: String,
    pub mode_int: i16,
    pub replay: bool,
    pub user: Option<OsuUserCompact>,
    pub beatmap: Option<OsuBeatmapScore>,
    pub beatmapset: Option<OsuBeatmapSetScore>,
    #[serde(rename = "match")]
    pub osu_match: Option<OsuScoreMatch>,
}

#[derive(Deserialize, Debug)]
pub struct OsuLeaderboard {
    pub scores: Vec<OsuScore>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct OsuUserCompact {
    pub avatar_url: String,
    pub country_code: String,
    pub default_group: String,
    pub id: u64,
    #[serde(deserialize_with = "datetime::deserialize_bool::deserialize")]
    pub is_active: bool,
    #[serde(deserialize_with = "datetime::deserialize_bool::deserialize")]
    pub is_bot: bool,
    #[serde(deserialize_with = "datetime::deserialize_bool::deserialize")]
    pub is_deleted: bool,
    #[serde(deserialize_with = "datetime::deserialize_bool::deserialize")]
    pub is_online: bool,
    #[serde(deserialize_with = "datetime::deserialize_bool::deserialize")]
    pub is_supporter: bool,
    #[serde(deserialize_with = "datetime::deserialize_bool::deserialize")]
    pub pm_friends_only: bool,
    pub username: String,
    // last_visit & profile_colour skipped
}

#[derive(Deserialize, Debug, Clone)]
pub struct OsuUserStatistics {
    pub count_300: Option<u32>,
    pub count_100: Option<u32>,
    pub count_50: Option<u32>,
    pub count_miss: Option<u32>,

    pub country_rank: Option<u32>,

    pub pp: f32,
    pub global_rank: u32,

    #[serde(deserialize_with = "datetime::deserialize_bool::deserialize")]
    pub is_ranked: bool,
    pub user: OsuUser,
}

#[derive(Deserialize, Debug, Clone, Copy)]
pub struct OsuUserExtendedStatisticsLevel {
    pub current: u32,
    pub progress: u32,
}

#[derive(Deserialize, Debug, Clone, Copy)]
pub struct OsuUserExtendedStatisticsGradeCounts {
    pub ss: u32,
    pub ssh: u32,
    pub s: u32,
    pub sh: u32,
    pub a: u32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct OsuUserExtendedStatistics {
    pub global_rank: Option<u32>,
    pub country_rank: Option<u32>,
    pub play_count: Option<u32>,
    pub hit_accuracy: Option<f32>,
    pub pp: f32,
    pub level: OsuUserExtendedStatisticsLevel,
    pub play_time: u32,
    pub grade_counts: OsuUserExtendedStatisticsGradeCounts,
}

#[derive(Deserialize, Debug, Clone)]
pub struct OsuUserExtended {
    pub id: i64,
    pub username: String,
    pub country_code: String,
    pub cover_url: String,
    pub avatar_url: String,
    pub discord: Option<String>,
    #[serde(deserialize_with = "datetime::deserialize_bool::deserialize")]
    pub has_supported: bool,
    pub interests: Option<String>,
    #[serde(deserialize_with = "datetime::deserialize::deserialize")]
    pub join_date: DateTime<Utc>,
    // Kudosu skipped
    pub location: Option<String>,
    pub max_blocks: i32,
    pub max_friends: i32,
    pub occupation: Option<String>,
    pub playmode: OsuGameMode,
    pub statistics: OsuUserExtendedStatistics,
    pub rank_highest: Option<OsuUserExtendedRankHighest>,
    pub user_achievements: Vec<OsuUserExtendedAchievement>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct OsuUserExtendedAchievement {
    #[serde(deserialize_with = "datetime::deserialize::deserialize")]
    pub achieved_at: DateTime<Utc>,
    pub achievement_id: u32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct OsuUserExtendedRankHighest {
    pub rank: u32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GetUsersResponse {
    pub users: Vec<OsuUser>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct OsuUser {
    pub id: i64,
    pub username: String,
    pub profile_colour: Option<String>,
    pub avatar_url: String,
    pub country_code: String,
    #[serde(deserialize_with = "datetime::deserialize_bool::deserialize")]
    pub is_active: bool,
    #[serde(deserialize_with = "datetime::deserialize_bool::deserialize")]
    pub is_bot: bool,
    #[serde(deserialize_with = "datetime::deserialize_bool::deserialize")]
    pub is_deleted: bool,
    #[serde(deserialize_with = "datetime::deserialize_bool::deserialize")]
    pub is_online: bool,
    #[serde(deserialize_with = "datetime::deserialize_bool::deserialize")]
    pub is_supporter: bool,
}

mod utils {
    pub fn cut(mut source: &str, n: usize) -> impl Iterator<Item = &str> {
        std::iter::from_fn(move || {
            if source.is_empty() {
                None
            } else {
                let end_idx = source
                    .char_indices()
                    .nth(n - 1)
                    .map_or_else(|| source.len(), |(idx, c)| idx + c.len_utf8());

                let (split, rest) = source.split_at(end_idx);

                source = rest;

                Some(split)
            }
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum UserId {
    Username(String),
    Id(i64),
}

/// Attemps to parse i64 from string
/// if failed defaults to username string
impl From<&str> for UserId {
    fn from(value: &str) -> Self {
        match value.parse::<i64>() {
            Ok(v) => Self::Id(v),
            Err(_) => Self::Username(value.to_owned()),
        }
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UserId::Username(v) => write!(f, "{v}"),
            UserId::Id(v) => write!(f, "{v}"),
        }
    }
}

// Get User Scores
pub enum ScoresType {
    Best,
    Firsts,
    Recent,
    Pinned,
}

impl fmt::Display for ScoresType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScoresType::Best => write!(f, "best"),
            ScoresType::Firsts => write!(f, "firsts"),
            ScoresType::Recent => write!(f, "recent"),
            ScoresType::Pinned => write!(f, "pinned"),
        }
    }
}

pub struct GetUserScores {
    pub user_id: i64,
    pub kind: ScoresType,
    pub include_fails: Option<bool>,
    pub mode: Option<OsuGameMode>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl GetUserScores {
    pub fn new(user_id: i64, kind: ScoresType) -> Self {
        Self {
            user_id,
            kind,
            include_fails: None,
            mode: None,
            limit: None,
            offset: None,
        }
    }

    pub fn limit(mut self, limit: i32) -> Self {
        self.limit = Some(limit);

        self
    }

    pub fn mode(mut self, mode: OsuGameMode) -> Self {
        self.mode = Some(mode);

        self
    }

    pub fn include_fails(mut self, include: bool) -> Self {
        self.include_fails = Some(include);
        self
    }
}

#[derive(PartialEq)]
pub enum RankingKind {
    Charts,
    Country,
    Performance,
    Score,
}

impl fmt::Display for RankingKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RankingKind::Charts => write!(f, "charts"),
            RankingKind::Country => write!(f, "country"),
            RankingKind::Performance => write!(f, "performance"),
            RankingKind::Score => write!(f, "score"),
        }
    }
}

pub enum RankingFilter {
    All,
    Friends,
}

impl fmt::Display for RankingFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RankingFilter::All => write!(f, "all"),
            RankingFilter::Friends => write!(f, "friends"),
        }
    }
}

pub struct GetRanking {
    pub mode: OsuGameMode,
    pub kind: RankingKind,
    pub filter: RankingFilter,
    pub country: Option<String>,
    pub page: Option<u32>,
    // Cursor
    // Country
    // Variant
}

#[derive(Deserialize, Debug)]
pub struct Rankings {
    pub ranking: Vec<OsuUserStatistics>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct BeatmapUserScore {
    pub scores: Vec<OsuScoreLazer>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ScoresBatchCursor {
    pub id: i64,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ScoresBatch {
    pub scores: Vec<OsuScoreLazer>,
    pub cursor_string: String,
    // pub cursor: ScoresBatchCursor,
}

#[test]
fn test_userid_conversion() {
    assert_eq!(UserId::from("486c"), UserId::Username("486c".to_owned()));
    assert_eq!(UserId::from("486"), UserId::Id(486));
    assert_eq!(UserId::from("1234567"), UserId::Id(1234567));
    assert_eq!(
        UserId::from("32178318LoPiJ"),
        UserId::Username("32178318LoPiJ".to_owned())
    );
}

#[inline]
pub fn ar_to_ms(ar: f32) -> f32 {
    if ar > 5.0 {
        1200.0 - 750.0 * (ar - 5.0) / 5.0
    } else if ar < 5.0 {
        1200.0 + 600.0 * (5.0 - ar) / 5.0
    } else {
        1200.0
    }
}

#[inline]
pub fn ms_to_ar(ms: f32) -> f32 {
    if ms < 1200.0 {
        ((ms * 5.0 - 1200.0 * 5.0) / (450.0 - 1200.0)) + 5.0
    } else if ms > 1200.0 {
        5.0 - ((1200.0 * 5.0 - ms * 5.0) / (1200.0 - 1800.0))
    } else {
        5.0
    }
}

pub enum HitWindow {
    Osu(f64, f64, f64),
    Mania(f64, f64, f64, f64, f64),
    Taiko(f64, f64, f64),
    Fruits,
}

#[inline]
pub fn hit_window(od: f64, mode: &OsuGameMode) -> HitWindow {
    match mode {
        OsuGameMode::Fruits => HitWindow::Fruits,
        OsuGameMode::Mania => HitWindow::Mania(
            16.0,
            64.0 - 3.0 * od,
            97.0 - 3.0 * od,
            127.0 - 3.0 * od,
            151.0 - 3.0 * od,
        ),
        OsuGameMode::Osu => HitWindow::Osu(80.0 - 6.0 * od, 140.0 - 8.0 * od, 200.0 - 10.0 * od),
        OsuGameMode::Taiko => {
            let great = 50.0 - 3.0 * od;

            let (ok, miss) = if od <= 5.0 {
                (120.0 - 8.0 * od, 135.0 - 8.0 * od)
            } else {
                (110.0 - 6.0 * od, 120.0 - 5.0 * od)
            };

            HitWindow::Taiko(great, ok, miss)
        }
    }
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
            HitWindow::Mania(max, c300, c200, c100, c50) => {
                Self::Mania(max / rhs, c300 / rhs, c200 / rhs, c100 / rhs, c50 / rhs)
            }
            HitWindow::Taiko(great, ok, miss) => Self::Taiko(great / rhs, ok / rhs, miss / rhs),
            HitWindow::Fruits => Self::Fruits,
        }
    }
}
