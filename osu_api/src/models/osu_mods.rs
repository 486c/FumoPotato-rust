use std::{fmt::Display, str::FromStr};

use serde::Deserialize;

use crate::{error::OsuApiError, models::OsuGameMode};

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
                return true;
            }
        }

        false
    }

    pub fn into_rosu_mods(&self, gamemode: OsuGameMode) -> rosu_mods::GameMods {
        let mut result = rosu_mods::GameMods::new();

        self.mods.iter().for_each(|x| {
            let osu_mod = rosu_mods::GameMod::new(&x.acronym, gamemode.into());

            result.insert(osu_mod)
        });

        result
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

            mods.push(OsuModLazer {
                acronym: chunk,
                settings: None,
            })
        }

        Ok(Self { mods })
    }
}

impl Display for OsuModsLazer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.mods.is_empty() {
            for mod_lazer in &self.mods {
                write!(f, "{mod_lazer}")?
            }

            if let Some(speed) = self.speed_changes() {
                write!(f, " (x{})", speed)?;
            }
        } else {
            write!(f, "NM")?
        }

        Ok(())
    }
}

impl Default for OsuModsLazer {
    fn default() -> Self {
        Self {
            mods: vec![OsuModLazer {
                acronym: "NM".to_string(),
                settings: None,
            }],
        }
    }
}

#[test]
fn test_osu_mods_lazer_formatting() {
    let t1 = OsuModsLazer::default();
    assert_eq!(format!("{}", t1), "NM");

    let t2 = OsuModsLazer {
        mods: vec![
            OsuModLazer {
                acronym: "DT".to_string(),
                settings: None,
            },
            OsuModLazer {
                acronym: "HR".to_string(),
                settings: None,
            },
            OsuModLazer {
                acronym: "HD".to_string(),
                settings: None,
            },
            OsuModLazer {
                acronym: "NF".to_string(),
                settings: None,
            },
        ],
    };
    assert_eq!(format!("{}", t2), "DTHRHDNF");

    let t3 = OsuModsLazer {
        mods: vec![OsuModLazer {
            acronym: "DT".to_string(),
            settings: Some(OsuModLazerSettings {
                speed_change: Some(1.2),
            }),
        }],
    };
    assert_eq!(format!("{}", t3), "DT (x1.2)");
}
