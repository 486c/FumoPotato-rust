use std::{fmt::Display, str::FromStr};

use serde::Deserialize;

use crate::error::OsuApiError;

/// Single mod
#[derive(Deserialize, Debug, Clone)]
pub struct OsuModLazer {
    acronym: String,
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
    mods: Vec<OsuModLazer>,
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

            mods.push(OsuModLazer { acronym: chunk })
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

#[test]
fn test_mods_creation() {
    let mods = OsuModsLazer::from_str("CLDTHR").unwrap();

    assert!(&format!("{mods}") == "CLDTHR");
}
