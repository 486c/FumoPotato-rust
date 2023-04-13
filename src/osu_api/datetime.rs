use chrono::{DateTime, NaiveDateTime, Utc};
use serde::de;
use std::fmt;

pub fn deserialize_datetime<'de, D>(d: D) -> Result<DateTime<Utc>, D::Error>
where
    D: de::Deserializer<'de>,
{
    struct LocalDateTimeVisitor;

    impl<'de> de::Visitor<'de> for LocalDateTimeVisitor {
        type Value = DateTime<Utc>;

        fn expecting(
            &self, 
            formatter: &mut fmt::Formatter
        ) -> fmt::Result {
            write!(formatter, "a datetime string")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                // Checking both datetime formats because osu!api loves
                // to change it randomly

                let parse = NaiveDateTime::parse_from_str(
                    value,
                    "%Y-%m-%dT%H:%M:%SZ"
                );

                if let Ok(ndt) = parse {
                    return Ok(DateTime::from_utc(ndt, Utc));
                }

                let parse = NaiveDateTime::parse_from_str(
                    value, 
                    "%Y-%m-%dT%H:%M:%S%z"
                );

                if let Ok(ndt) = parse {
                    return Ok(DateTime::from_utc(ndt, Utc));
                }
                

                Err(E::custom(
                    format!("Failed to parse datetime {value}")
                ))
            }
    }

    d.deserialize_str(LocalDateTimeVisitor)
}

