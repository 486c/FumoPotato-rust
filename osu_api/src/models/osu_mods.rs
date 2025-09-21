use std::{fmt::Display, str::FromStr};

use serde::Deserialize;

use crate::error::OsuApiError;

#[derive(Deserialize, Debug, Clone)]
pub struct OsuModLazerSettings {
    speed_change: Option<f32>,
}

/// Single mod
#[derive(Deserialize, Debug, Clone)]
pub struct OsuModLazer {
    pub acronym: String,
    pub settings: Option<OsuModLazerSettings>,
}

impl Display for OsuModLazer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.acronym)
    }
}

/// Multiple mods
#[derive(Deserialize, Debug, Clone)]
#[serde(transparent)]
pub struct OsuModsLazer {
    pub mods: Vec<OsuModLazer>,
}

impl OsuModsLazer {
    pub fn speed_changes(&self) -> Option<f32> {
        for osu_mod in &self.mods {
            if let Some(settings) = &osu_mod.settings {
                if let Some(speed_change) = settings.speed_change {
                    return Some(speed_change);
                }
            }
        }

        None
    }

    pub fn contains(&self, acronym: &str) -> bool {
        for osu_mod in &self.mods {
            if osu_mod.acronym == acronym {
                return true
            }
        }

        false
    }
}

impl FromStr for OsuModsLazer {
    type Err = OsuApiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut ch = s.chars().peekable();

        // TODO Check size
        let mut mods = Vec::new();

        while ch.peek().is_some() {
            // TODO refactor
            let chunk: String = ch.by_ref().take(2).collect();

            mods.push(OsuModLazer { acronym: chunk, settings: None })
        }

        Ok(Self { mods })
    }
}

impl Display for OsuModsLazer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for mod_lazer in &self.mods {
            write!(f, "{mod_lazer}")?
        }

        Ok(())
    }
}

impl Default for OsuModsLazer {
    fn default() -> Self {
        Self {
            mods: vec![OsuModLazer { acronym: "NM".to_string(), settings: None }],
        }
    }
}
