use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::datetime;

use super::{osu_mods::OsuModsLazer, OsuGrade, OsuUserCompact};

#[derive(Deserialize, Debug)]
pub struct StatisticsLazer {
    pub ok: Option<u32>,
    pub meh: Option<u32>,
    pub miss: Option<u32>,
    pub great: Option<u32>,
}

#[derive(Deserialize, Debug)]
pub struct OsuScoreLazer {
    #[serde(deserialize_with = "datetime::deserialize_bool::deserialize")]
    pub ranked: bool,
    #[serde(deserialize_with = "datetime::deserialize_bool::deserialize")]
    pub preserve: bool,
    pub beatmap_id: i32,

    pub mods: OsuModsLazer,
    pub best_id: Option<u32>,
    pub id: u32,
    pub rank: OsuGrade,

    pub statistics: StatisticsLazer,

    #[serde(rename = "type")]
    pub kind: String, // TODO enum

    pub user_id: u64,
    pub accuracy: f32,

    pub pp: Option<f32>,

    pub total_score: u64,
    pub legacy_total_score: u64,
    pub max_combo: u32,

    pub user: OsuUserCompact,

    #[serde(deserialize_with = "datetime::deserialize::deserialize")]
    pub ended_at: DateTime<Utc>,
}

#[derive(Deserialize, Debug)]
pub struct OsuLeaderboardLazer {
    pub scores: Vec<OsuScoreLazer>,
}
