use chrono::{DateTime, NaiveDateTime, Utc};
use serde::de;
use std::fmt;

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
        // Checking both datetime formats because osu!api loves
        // to change it randomly

        let parse = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%SZ");

        if let Ok(ndt) = parse {
            return Ok(DateTime::from_naive_utc_and_offset(ndt, Utc));
        }

        let parse = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%z");

        if let Ok(ndt) = parse {
            return Ok(DateTime::from_naive_utc_and_offset(ndt, Utc));
        }

        Err(E::custom(format!("Failed to parse utc datetime {value}")))
    }
}

pub mod deserialize {
    use chrono::{DateTime, Utc};
    use serde::Deserializer;

    use super::LocalDateTimeVisitor;

    pub fn deserialize<'de, D: Deserializer<'de>>(
        d: D,
    ) -> Result<DateTime<Utc>, D::Error> {
        d.deserialize_str(LocalDateTimeVisitor)
    }
}

pub mod deserialize_option {
    use std::fmt;

    use chrono::{DateTime, Utc};
    use serde::{
        de::{Error, Visitor},
        Deserializer,
    };

    use super::LocalDateTimeVisitor;

    struct OptionLocalDateTimeVisitor;

    pub fn deserialize<'de, D: Deserializer<'de>>(
        d: D,
    ) -> Result<Option<DateTime<Utc>>, D::Error> {
        d.deserialize_option(OptionLocalDateTimeVisitor)
    }

    impl<'de> Visitor<'de> for OptionLocalDateTimeVisitor {
        type Value = Option<DateTime<Utc>>;

        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("an optional datetime string")
        }

        #[inline]
        fn visit_some<D: Deserializer<'de>>(
            self,
            d: D,
        ) -> Result<Self::Value, D::Error> {
            d.deserialize_str(LocalDateTimeVisitor).map(Some)
        }

        #[inline]
        fn visit_none<E: Error>(self) -> Result<Self::Value, E> {
            self.visit_unit()
        }

        #[inline]
        fn visit_unit<E: Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
    }
}

pub mod deserialize_bool {
    use std::fmt;

    use serde::{de, Deserializer};

    struct OsuBoolVisitor;

    pub fn deserialize<'de, D: Deserializer<'de>>(
        d: D,
    ) -> Result<bool, D::Error> {
        d.deserialize_any(OsuBoolVisitor)
    }

    impl<'de> de::Visitor<'de> for OsuBoolVisitor {
        type Value = bool;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            write!(formatter, "a 1,0 or false, true string")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            match value {
                "true" => Ok(true),
                "false" => Ok(false),
                _ => Err(E::custom("expected: true, false".to_string())),
            }
        }

        fn visit_u8<E>(self, value: u8) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            match value {
                1 => Ok(true),
                0 => Ok(false),
                _ => Err(E::custom("expected: 1, 0".to_string())),
            }
        }

        fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value)
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            match value {
                1 => Ok(true),
                0 => Ok(false),
                _ => Err(E::custom("expected: 1, 0".to_string())),
            }
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            match value {
                1 => Ok(true),
                0 => Ok(false),
                _ => Err(E::custom("expected: 1, 0".to_string())),
            }
        }
    }
}
