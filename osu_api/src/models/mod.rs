pub mod osu_mods;
pub mod osu_leaderboard;
pub mod osu_matches;

use crate::datetime;

use crate::error::OsuApiError;

use chrono::prelude::*;

use serde::Deserialize;
use serde::de::{ Unexpected, Visitor, Deserializer, Error, SeqAccess };
use thiserror::Error;

use std::string::ToString;
use std::str::FromStr;
use std::fmt;

use bitflags::bitflags;

#[derive(Error, Debug, Deserialize)]
pub struct ApiError {
    pub error: Option<String>
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

    fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
        match v {
            "graveyard" => Ok(RankStatus::Graveyard),
            "wip" => Ok(RankStatus::Wip),
            "pending" => Ok(RankStatus::Pending),
            "ranked" => Ok(RankStatus::Ranked),
            "approved" => Ok(RankStatus::Approved),
            "qualified" => Ok(RankStatus::Qualified),
            "loved" => Ok(RankStatus::Loved),
            _ => return Err(Error::invalid_value(Unexpected::Str(&v), &r#"ranked, graveyard, wip and other"#))
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

impl<'de> Visitor<'de> for OsuRankVisitor {
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
            _ => return Err(
                Error::invalid_value(
                    Unexpected::Str(v),
                    &r#""XH", "SH", "X", "S", "A", "B", "C", "D" or "F""#)
                ),
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
        if self.contains(OsuMods::AUTOPILOT) {
            res.push_str("AP")
        }
        if self.contains(OsuMods::KEY1) {
            res.push_str("K1")
        }
        if self.contains(OsuMods::KEY2) {
            res.push_str("K2")
        }
        if self.contains(OsuMods::KEY3) {
            res.push_str("K3")
        }
        if self.contains(OsuMods::KEY4) {
            res.push_str("K4")
        }
        if self.contains(OsuMods::KEY5) {
            res.push_str("K5")
        }
        if self.contains(OsuMods::KEY6) {
            res.push_str("K6")
        }
        if self.contains(OsuMods::KEY7) {
            res.push_str("K7")
        }
        if self.contains(OsuMods::KEY8) {
            res.push_str("K8")
        }
        if self.contains(OsuMods::KEY9) {
            res.push_str("K9")
        }
        if self.contains(OsuMods::RANDOM) {
            res.push_str("RD")
        }
        if self.contains(OsuMods::KEYCOOP) {
            res.push_str("2P")
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
            let mods = Self::from_acronym_str(abbrev); 
            match mods {
                Some(m) => flags = flags | m,
                None => {},
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

    fn visit_seq<A: SeqAccess<'de>>(
        self, 
        mut seq: A
    ) -> Result<Self::Value, A::Error> {
        let mut mods = OsuMods::default();

        while let Some(next) = seq.next_element()? {
            mods |= next;
        }

        Ok(mods)
    }

    fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
        let mods = match OsuMods::from_acronym_str(v) {
            Some(m) => m,
            None => return Err(
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

#[derive(Debug, Clone)]
pub enum OsuGameMode {
    Fruits,
    Mania,
    Osu,
    Taiko
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
}

struct OsuGameModeVisitor;

impl<'de> Visitor<'de> for OsuGameModeVisitor {
    type Value = OsuGameMode;

    #[inline]
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("valid string")
    }

    fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
        match v {
            "osu" => Ok(OsuGameMode::Osu),
            "fruits" => Ok(OsuGameMode::Fruits),
            "taiko" => Ok(OsuGameMode::Taiko),
            "mania" => Ok(OsuGameMode::Mania),
            _ => Err(Error::invalid_value(
                Unexpected::Str(v),
                &"osu, fruits, taiko or mania"
            ))
        }
    }
}

impl<'de> Deserialize<'de> for OsuGameMode {
    #[inline]
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_any(OsuGameModeVisitor)
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
    pub count50: i32,
    #[serde(rename = "count_100")]
    pub count100: i32,
    #[serde(rename = "count_300")]
    pub count300: i32,
    #[serde(rename = "count_geki")]
    pub countgeki: Option<i32>,
    #[serde(rename = "count_katu")]
    pub countkatu: Option<i32>,
    #[serde(rename = "count_miss")]
    pub countmiss: i32,
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct OsuBeatmapsetCompact {
    title: String,
    artist: String,
    creator: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct OsuBeatmap {
    pub beatmapset_id: i32,
    pub id: i32,
    pub mode: String,

    pub version: String,

    pub beatmapset: OsuBeatmapsetCompact,

    pub max_combo: Option<i32>,
    pub status: RankStatus,
}

impl OsuBeatmap {
    pub fn metadata(&self) -> String {
        format!(
            "{} - {} [{}]", 
            self.beatmapset.artist, 
            self.beatmapset.title, 
            self.version
        )
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

#[derive(Deserialize, Clone, Debug)]
pub struct OsuScoreMatch {
    pub slot: i32,
    pub team: String,
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

#[derive(Deserialize, Debug, Clone)]
pub struct OsuUserExtendedStatistics {
    pub global_rank: u32,
    pub country_rank: u32,
    pub pp: f32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct OsuUserExtended {
    pub id: i64,
    pub username: String,
    pub country_code: String,
    pub cover_url: String,
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
                    .map_or_else(
                        || source.len(), 
                        |(idx, c)| idx + c.len_utf8() 
                );

                let (split, rest) = source.split_at(end_idx);

                source = rest;

                Some(split)
            }
        })
    }
}

pub enum UserId {
    Username(String),
    Id(i64)
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
}

impl fmt::Display for ScoresType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScoresType::Best => write!(f, "best"),
            ScoresType::Firsts => write!(f, "firsts"),
            ScoresType::Recent => write!(f, "recent"),
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
    Friends
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
    pub page: Option<u32>
    // Cursor
    // Country
    // Variant
}

#[derive(Deserialize, Debug)]
pub struct Rankings {
    pub ranking: Vec<OsuUserStatistics>,
}