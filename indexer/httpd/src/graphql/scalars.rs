use async_graphql::{Scalar, ScalarType, Value};

/// A custom DateTime scalar that ensures ISO8601 compliance with timezone information
#[derive(Clone, Debug)]
pub struct Iso8601DateTime(pub chrono::NaiveDateTime);

#[Scalar(name = "DateTime")]
impl ScalarType for Iso8601DateTime {
    fn parse(value: Value) -> async_graphql::InputValueResult<Self> {
        match value {
            Value::String(s) => {
                // Try parsing as RFC3339 first (with timezone)
                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&s) {
                    Ok(Self(dt.naive_utc()))
                } else if let Ok(dt) =
                    chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S")
                {
                    // Parse basic ISO format without timezone, assume UTC
                    Ok(Self(dt))
                } else if let Ok(dt) =
                    chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S%.f")
                {
                    // Parse ISO format with microseconds but no timezone, assume UTC
                    Ok(Self(dt))
                } else {
                    Err(async_graphql::InputValueError::custom(
                        "Invalid datetime format",
                    ))
                }
            },
            _ => Err(async_graphql::InputValueError::custom("Expected string")),
        }
    }

    fn to_value(&self) -> Value {
        // Convert to UTC DateTime and format as ISO8601 with Z suffix
        let utc_dt = chrono::DateTime::<chrono::Utc>::from_utc(self.0, chrono::Utc);
        Value::String(utc_dt.to_rfc3339_opts(chrono::SecondsFormat::Micros, true))
    }
}

impl From<chrono::NaiveDateTime> for Iso8601DateTime {
    fn from(dt: chrono::NaiveDateTime) -> Self {
        Self(dt)
    }
}

impl From<Iso8601DateTime> for chrono::NaiveDateTime {
    fn from(dt: Iso8601DateTime) -> Self {
        dt.0
    }
}
