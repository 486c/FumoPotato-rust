use serde::Deserialize;

use crate::models::{OsuGameMode, OsuMods};


#[derive(Debug, Clone, Deserialize)]
pub struct FallbackScoreStatsMods {
    pub number: OsuMods,
    //pub array
}

#[derive(Debug, Clone, Deserialize)]
pub struct FallbackScoreStats {
    pub score: i64,
    pub performance: f32,
    pub combo: u32,
    pub accuracy: f32,
    pub rank: String,
    pub mods: FallbackScoreStatsMods,

}

#[derive(Debug, Clone, Deserialize)]
pub struct FallabackScoreStatus {
    #[serde(rename = "isReplayAvailable")]
    replay: bool,
    #[serde(rename = "isPerfect")]
    perfect: bool,
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
pub struct FallbackScore {
    pub position: u32,
    pub id: i64,
    pub beatmap: i64,
    pub player: i64,
    //pub date

    pub status: FallabackScoreStatus,
    pub counts: FallabackScoreCounts,
    pub stats: FallbackScoreStats,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FallbackBeatmapScores {
    pub ruleset: OsuGameMode,
    pub count: u32,
    pub items: Vec<FallbackScore>
}
