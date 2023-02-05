use chrono::{DateTime, NaiveDateTime, Utc};
use serde::de;
use std::fmt;

pub fn deserialize_local_datetime<'de, D>(d: D) -> Result<DateTime<Utc>, D::Error>
where
    D: de::Deserializer<'de>,
{
    struct LocalDateTimeVisitor;

    impl<'de> de::Visitor<'de> for LocalDateTimeVisitor {
        type Value = DateTime<Utc>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            write!(formatter, "a datetime string")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%SZ") {
                    Ok(ndt) => Ok(DateTime::from_utc(ndt, Utc)),
                    Err(e) => Err(E::custom(format!("Parse error {e} for {value}"))),
                }
            }
    }

    d.deserialize_str(LocalDateTimeVisitor)
}

