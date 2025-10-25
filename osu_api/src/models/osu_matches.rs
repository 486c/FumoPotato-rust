use std::fmt;

use chrono::{DateTime, Utc};
use serde::{de, Deserialize, Deserializer};

use crate::datetime;

use super::{OsuBeatmap, OsuGameMode, OsuMods, OsuScore};

/// Used in `/matches` endpoint
#[derive(Debug, Clone, Deserialize)]
pub struct OsuMatchCompact {
    pub id: i64,
    #[serde(deserialize_with = "datetime::deserialize::deserialize")]
    pub start_time: DateTime<Utc>,
    #[serde(deserialize_with = "datetime::deserialize_option::deserialize")]
    pub end_time: Option<DateTime<Utc>>,
    pub name: String,
}

/// Used in `/matches` endpoint
#[derive(Debug, Clone, Deserialize)]
pub struct OsuMatchContainer {
    pub matches: Vec<OsuMatchCompact>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OsuMatch {
    pub id: i64,
    pub name: String,
    #[serde(deserialize_with = "datetime::deserialize::deserialize")]
    pub start_time: DateTime<Utc>,
    #[serde(deserialize_with = "datetime::deserialize_option::deserialize")]
    pub end_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum OsuMatchEventKind {
    #[serde(rename = "host-changed")]
    HostChanged,
    #[serde(rename = "match-created")]
    MatchCreated,
    #[serde(rename = "match-disbanded")]
    MatchDisbanded,
    #[serde(rename = "other")]
    Other,
    #[serde(rename = "player-joined")]
    PlayerJoined,
    #[serde(rename = "player-kicked")]
    PlayerKicked,
    #[serde(rename = "player-left")]
    PlayerLeft,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OsuMatchEvent {
    pub detail: OsuMatchEventDetails,
    pub game: Option<OsuMatchGame>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OsuMatchEventDetails {
    #[serde(rename = "type")]
    pub kind: OsuMatchEventKind,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OsuMatchGame {
    pub id: i64,
    pub beatmap: Option<OsuBeatmap>,
    pub beatmap_id: i64,
    #[serde(deserialize_with = "datetime::deserialize::deserialize")]
    pub start_time: DateTime<Utc>,
    #[serde(deserialize_with = "datetime::deserialize_option::deserialize")]
    pub end_time: Option<DateTime<Utc>>,
    pub mode: OsuGameMode,
    pub mods: OsuMods,
    #[serde(rename = "scoring_type")]
    pub scoring_kind: ScoringKind,
    #[serde(rename = "team_type")]
    pub team_kind: TeamKind,
    pub scores: Vec<OsuScore>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OsuMatchGet {
    #[serde(rename = "match")]
    pub osu_match: OsuMatch,
    pub events: Vec<OsuMatchEvent>,
    pub first_event_id: i64,
    pub latest_event_id: i64,
}

impl OsuMatchGet {
    pub fn is_match_disbanded(&self) -> bool {
        if self.events.is_empty() {
            true
        } else {
            for i in (0..self.events.len()).rev() {
                if self.events[i].detail.kind
                    == OsuMatchEventKind::MatchDisbanded
                {
                    return true;
                }
            }

            false
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum ScoringKind {
    Accuracy,
    Combo,
    Score,
    ScoreV2,
}

impl ScoringKind {
    pub fn as_u8(&self) -> u8 {
        *self as u8
    }
}

struct ScoringKindVisitor;

impl de::Visitor<'_> for ScoringKindVisitor {
    type Value = ScoringKind;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "accuracy, combo, score or scorev2")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match value {
            "accuracy" => Ok(ScoringKind::Accuracy),
            "combo" => Ok(ScoringKind::Combo),
            "score" => Ok(ScoringKind::Score),
            "scorev2" => Ok(ScoringKind::ScoreV2),
            _ => Err(serde::de::Error::custom(format!(
                "Invalid variant: {}",
                value
            ))),
        }
    }
}

impl<'de> Deserialize<'de> for ScoringKind {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        d.deserialize_any(ScoringKindVisitor)
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum TeamKind {
    HeadToHead,
    TagCoop,
    TagTeamVs,
    TeamVs,
}

impl TeamKind {
    pub fn as_u8(&self) -> u8 {
        *self as u8
    }
}

struct TeamKindVisitor;

impl<'de> Deserialize<'de> for TeamKind {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        d.deserialize_any(TeamKindVisitor)
    }
}

impl de::Visitor<'_> for TeamKindVisitor {
    type Value = TeamKind;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "head-to-head, tag-coop, tag-team-vs or team-vs")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match value {
            "head-to-head" => Ok(TeamKind::HeadToHead),
            "tag-coop" => Ok(TeamKind::TagCoop),
            "tag-team-vs" => Ok(TeamKind::TagTeamVs),
            "team-vs" => Ok(TeamKind::TeamVs),
            _ => Err(serde::de::Error::custom(format!(
                "Invalid variant: {}",
                value
            ))),
        }
    }
}
