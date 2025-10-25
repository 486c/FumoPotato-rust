use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer,
};
use std::fmt;

#[derive(Debug, Clone, Deserialize)]
pub struct FallbackOsuMod {
    pub acronym: String,

    #[serde(deserialize_with = "deserialize")]
    pub speed: Option<f32>,
}

pub fn deserialize<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<Option<f32>, D::Error> {
    d.deserialize_any(FallbackOsuModVisitor)
}

struct FallbackOsuModVisitor;

impl Visitor<'_> for FallbackOsuModVisitor {
    type Value = Option<f32>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("an speed f32 value or 0")
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Some(value as f32))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if value == 0 {
            Ok(None)
        } else {
            Ok(Some(value as f32))
        }
    }
}
