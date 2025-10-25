use std::fmt;

use chrono::{DateTime, Utc};
use serde::de;

struct LocalDateTimeVisitor;

impl de::Visitor<'_> for LocalDateTimeVisitor {
    type Value = DateTime<Utc>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a datetime string")
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match DateTime::from_timestamp(value as i64, 0) {
            Some(v) => Ok(v),
            None => Err(E::custom(format!(
                "Failed to parse utc datetime from timestamp {value}"
            ))),
        }
    }
}

pub mod deserialize {
    use chrono::{DateTime, Utc};
    use serde::Deserializer;

    use super::LocalDateTimeVisitor;

    pub fn deserialize<'de, D: Deserializer<'de>>(
        d: D,
    ) -> Result<DateTime<Utc>, D::Error> {
        d.deserialize_any(LocalDateTimeVisitor)
    }
}
