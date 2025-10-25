use chrono::{DateTime, Utc};
use osu_fallback_mods::FallbackOsuMod;
use serde::Deserialize;

use crate::{
    datetime_timestamp,
    models::{OsuGameMode, OsuGrade, OsuMods},
};

pub mod osu_fallback_mods;

#[derive(Debug, Clone, Deserialize)]
pub struct FallbackScoreStatsMods {
    pub number: OsuMods,
    pub array: Vec<String>,
    pub difficulty: Vec<FallbackOsuMod>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FallbackScoreScore {
    pub lazer: i64,
    pub legacy: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FallbackScoreStats {
    pub score: FallbackScoreScore,
    pub performance: f32,
    pub combo: u32,
    pub accuracy: f32,
    pub rank: OsuGrade,
    pub mods: FallbackScoreStatsMods,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FallabackScoreStatus {
    // unused
    //#[serde(rename = "isReplayAvailable")]
    // replay: bool,
    //#[serde(rename = "isPerfect")]
    // perfect: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FallabackScoreCounts {
    #[serde(rename = "50")]
    pub x50: u32,
    #[serde(rename = "100")]
    pub x100: u32,
    #[serde(rename = "300")]
    pub x300: u32,
    #[serde(rename = "geki")]
    pub xgeki: u32,
    #[serde(rename = "katu")]
    pub xkatu: u32,
    #[serde(rename = "miss")]
    pub xmiss: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FallabackScoreBeatmap {
    // unused
    // id: i64
}

#[derive(Debug, Clone, Deserialize)]
pub struct FallabackScorePlayer {
    pub id: i64,
    pub username: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FallbackScore {
    pub position: u32,
    pub id: i64,

    #[serde(deserialize_with = "datetime_timestamp::deserialize::deserialize")]
    pub date: DateTime<Utc>,

    pub status: FallabackScoreStatus,
    pub counts: FallabackScoreCounts,
    pub player: FallabackScorePlayer,
    pub beatmap: FallabackScoreBeatmap,
    pub stats: FallbackScoreStats,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FallbackBeatmapScores {
    pub ruleset: OsuGameMode,
    pub count: u32,
    pub items: Vec<FallbackScore>,
}
