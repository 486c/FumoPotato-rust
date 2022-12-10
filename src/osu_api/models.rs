use crate::osu_api::datetime::deserialize_local_datetime;
use crate::osu_api::error::OsuApiError;

use chrono::prelude::*;

use eyre::Result;

use serde::Deserialize;
use serde::de::{ Unexpected, Visitor, Deserializer, Error, SeqAccess };

use std::string::ToString;
use std::str::FromStr;
use std::fmt;

use bitflags::bitflags;

#[derive(Debug, Eq, PartialEq)]
pub enum RankStatus {
    Graveyard = -2,
    Wip = -1,
    Pending = 0,
    Ranked = 1,
    Approved = 2,
    Qualified = 3,
    Loved = 4,
}

struct RankStatusVisitor;

impl<'de> Visitor<'de> for RankStatusVisitor {
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
            _ => return Err(
                Error::invalid_value(
                    Unexpected::Unsigned(v),
                    &r#"0, 1, 2, 3 or 4"#)
                ),
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
            _ => return Err(
                Error::invalid_value(
                    Unexpected::Signed(v),
                    &r#"-2, -1, 0, 1, 2, 3 or 4"#)
                ),
        }
    }
}

impl<'de> Deserialize<'de> for RankStatus {
    #[inline]
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_any(RankStatusVisitor)
    }
}

#[derive(Debug)]
pub enum OsuRank {
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

impl OsuRank {
    pub fn to_emoji(&self) -> &str {
        match self {
            OsuRank::GradeXH => "<:r_XH:1004444329365999766>",
            OsuRank::GradeSH => "<:r_SH:1004444326669066270>",
            OsuRank::GradeX => "<:r_X:1004444328082538546>",
            OsuRank::GradeS => "<:r_S:1004444324840349759>",
            OsuRank::GradeA => "<:r_A:1004444322365702204>",
            OsuRank::GradeB => "<:r_B:1004444032149233696>",
            OsuRank::GradeC => "<:r_C:1004444033524957235>",
            OsuRank::GradeD => "<:r_D:1004444323703701545>",
            OsuRank::GradeF => "<:r_D:1004444323703701545>",
        }
    }
}

impl fmt::Display for OsuRank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OsuRank::GradeXH => write!(f, "XH"),
            OsuRank::GradeSH => write!(f, "SH"),
            OsuRank::GradeX => write!(f, "X"),
            OsuRank::GradeS => write!(f, "S"),
            OsuRank::GradeA => write!(f, "A"),
            OsuRank::GradeB => write!(f, "B"),
            OsuRank::GradeC => write!(f, "C"),
            OsuRank::GradeD => write!(f, "D"),
            OsuRank::GradeF => write!(f, "F"),
        }
    }
}

struct OsuRankVisitor;

impl<'de> Visitor<'de> for OsuRankVisitor {
    type Value = OsuRank;

    #[inline]
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a valid rank string")
    }

    fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
        let rank = match v {
            "XH" => OsuRank::GradeXH,
            "SH" => OsuRank::GradeSH,
            "X" => OsuRank::GradeX,
            "S" => OsuRank::GradeS,
            "A" => OsuRank::GradeA,
            "B" => OsuRank::GradeB,
            "C" => OsuRank::GradeC,
            "D" => OsuRank::GradeD,
            "F" => OsuRank::GradeF,
            _ => return Err(
                Error::invalid_value(
                    Unexpected::Str(v),
                    &r#""XH", "SH", "X", "S", "A", "B", "C", "D" or "F""#)
                ),
        };

        Ok(rank)
    }
}

impl<'de> Deserialize<'de> for OsuRank {
    #[inline]
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_any(OsuRankVisitor)
    }
}

bitflags! {
    #[derive(Default)]
    pub struct OsuMods: u32 {
        const NOMOD = 0;
        const NOFAIL = 1;
        const EASY = 2;
        const TOUCHDEVICE = 4;
        const HIDDEN = 8;
        const HARDROCK = 16;
        const SUDDENDEATH = 32;
        const DOUBLETIME = 64;
        const RELAX = 128;
        const HALFTIME = 256;
        const NIGHTCORE = 512 | Self::DOUBLETIME.bits;
        const FLASHLIGHT = 1024;
        const SPUNOUT = 4096;
        const PERFECT = 16_384 | Self::SUDDENDEATH.bits;
        const FADEIN = 1_048_576;
        const SCOREV2 = 536_870_912;
        const MIRROR = 1_073_741_824;
    }
}

impl ToString for OsuMods {
    fn to_string(&self) -> String {
        let mut res = String::new();

        if self.is_empty() {
            res.push_str("NM");
            return res
        }

        if self.contains(OsuMods::NOFAIL) {
            res.push_str("NF")
        }
        if self.contains(OsuMods::EASY) {
            res.push_str("EZ")
        }
        if self.contains(OsuMods::TOUCHDEVICE) {
            res.push_str("TD")
        }
        if self.contains(OsuMods::HIDDEN) {
            res.push_str("HD")
        }
        if self.contains(OsuMods::DOUBLETIME) {
            if self.contains(OsuMods::NIGHTCORE) {
                res.push_str("NC")
            } else {
                res.push_str("DT")
            }
        }
        if self.contains(OsuMods::HALFTIME) {
            res.push_str("HT")
        }
        if self.contains(OsuMods::FLASHLIGHT) {
            res.push_str("FL")
        }
        if self.contains(OsuMods::HARDROCK) {
            res.push_str("HR")
        }
        if self.contains(OsuMods::SUDDENDEATH) {
            res.push_str("SD")
        }
        if self.contains(OsuMods::SPUNOUT) {
            res.push_str("SO")
        }
        if self.contains(OsuMods::PERFECT) {
            res.push_str("PF")
        }
        if self.contains(OsuMods::MIRROR) {
            res.push_str("MR")
        }

        res
    }
}

impl FromStr for OsuMods {
    type Err = OsuApiError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_uppercase();
        let mut flags = OsuMods::empty();

        for abbrev in utils::cut(&s, 2) {
            flags = match abbrev {
                "NM" => flags | OsuMods::NOMOD,
                "NF" => flags | OsuMods::NOFAIL,
                "EZ" => flags | OsuMods::EASY,
                "TD" => flags | OsuMods::TOUCHDEVICE,
                "HD" => flags | OsuMods::HIDDEN,
                "HR" => flags | OsuMods::HARDROCK,
                "SD" => flags | OsuMods::SUDDENDEATH,
                "DT" => flags | OsuMods::DOUBLETIME,
                "RX" => flags | OsuMods::RELAX,
                "HT" => flags | OsuMods::HALFTIME,
                "NC" => flags | OsuMods::NIGHTCORE,
                "FL" => flags | OsuMods::FLASHLIGHT,
                "SO" => flags | OsuMods::SPUNOUT,
                "PF" => flags | OsuMods::PERFECT,
                "FD" => flags | OsuMods::FADEIN,
                _ => flags,
            };
        };

        Ok(flags)
    }
}

struct OsuModsVisitor;

impl<'de> Visitor<'de> for OsuModsVisitor {
    type Value = OsuMods;

    #[inline]
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut mods = OsuMods::default();

        while let Some(next) = seq.next_element()? {
            mods |= next;
        }

        Ok(mods)
    }

    fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
        let mods = match v {
            "NM" => OsuMods::NOMOD,
            "NF" => OsuMods::NOFAIL,
            "EZ" => OsuMods::EASY,
            "TD" => OsuMods::TOUCHDEVICE,
            "HD" => OsuMods::HIDDEN,
            "HR" => OsuMods::HARDROCK,
            "SD" => OsuMods::SUDDENDEATH,
            "DT" => OsuMods::DOUBLETIME,
            "RX" => OsuMods::RELAX,
            "HT" => OsuMods::HALFTIME,
            "NC" => OsuMods::NIGHTCORE,
            "FL" => OsuMods::FLASHLIGHT,
            "SO" => OsuMods::SPUNOUT,
            "PF" => OsuMods::PERFECT,
            "FD" => OsuMods::FADEIN,
            _ => return Err(
                Error::invalid_value(
                    Unexpected::Str(v),
                    &r#"valid mods acronym"#)
                ),
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

#[derive(Deserialize, Debug)]
pub struct OauthResponse {
    pub token_type: String,
    pub expires_in: i32,
    pub access_token: String,
}

#[derive(Deserialize, Debug)]
pub struct OsuScoreStatistics {
    #[serde(rename = "count_50")]
    pub count50: i32,
    #[serde(rename = "count_100")]
    pub count100: i32,
    #[serde(rename = "count_300")]
    pub count300: i32,
    #[serde(rename = "count_geki")]
    pub countgeki: i32,
    #[serde(rename = "count_katu")]
    pub countkatu: i32,
    #[serde(rename = "count_miss")]
    pub countmiss: i32,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct OsuBeatmapsetCompact {
    title: String,
    artist: String,
    creator: String,
}

#[derive(Deserialize, Debug)]
pub struct OsuBeatmap {
    pub beatmapset_id: i32,
    pub id: i32,
    pub mode: String,

    pub version: String,

    pub beatmapset: OsuBeatmapsetCompact,

    pub max_combo: i32,
    pub ranked: RankStatus,
}

impl OsuBeatmap {
    pub fn metadata(&self) -> String {
        format!("{} - {} [{}]", self.beatmapset.artist, self.beatmapset.title, self.version)
    }
}

#[derive(Deserialize, Debug)]
pub struct OsuScore {
    pub id: i64,
    pub best_id: i64,
    pub user_id: i64,
    pub accuracy: f32,
    pub mods: OsuMods,
    pub score: i64,
    pub perfect: bool,
    pub passed: bool,
    pub pp: Option<f32>,

    #[serde(rename = "max_combo")]
    pub max_combo: i32,

    pub rank: OsuRank,

    #[serde(deserialize_with = "deserialize_local_datetime")]
    pub created_at: DateTime<Utc>,

    #[serde(rename = "statistics")]
    pub stats: OsuScoreStatistics,

    pub mode: String,
    pub mode_int: i16,
    pub replay: bool,
    pub user: OsuUserCompact,
}

#[derive(Deserialize, Debug)]
pub struct OsuLeaderboard {
    pub scores: Vec<OsuScore>,
}

#[derive(Deserialize, Debug)]
pub struct OsuUserCompact {
    pub avatar_url: String,
    pub country_code: String,
    pub default_group: String,
    pub id: i64,
    pub is_active: bool,
    pub is_bot: bool,
    pub is_deleted: bool,
    pub is_online: bool,
    pub is_supporter: bool,
    pub pm_friends_only: bool,
    pub username: String,
    // last_visit & profile_colour skipped
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
                    .map_or_else(|| source.len(), |(idx, c)| idx + c.len_utf8() 
                );

                let (split, rest) = source.split_at(end_idx);

                source = rest;

                Some(split)
            }
        })
    }
}

