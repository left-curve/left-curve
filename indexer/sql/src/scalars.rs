use {
    async_graphql::{InputValueError, InputValueResult, Scalar, ScalarType, Value},
    grug_types::Timestamp,
    sqlx::types::chrono::{DateTime as ChronoDateTime, NaiveDateTime},
};

/// A custom DateTime scalar that ensures ISO8601 compliance with timezone information
#[derive(Clone, Debug)]
pub struct Iso8601DateTime(pub NaiveDateTime);

#[Scalar(name = "DateTime")]
impl ScalarType for Iso8601DateTime {
    fn parse(value: Value) -> InputValueResult<Self> {
        let Value::String(s) = value else {
            return Err(InputValueError::custom("expected string"));
        };

        // Try parsing as RFC3339 first (with timezone).
        if let Ok(dt) = ChronoDateTime::parse_from_rfc3339(&s) {
            return Ok(Self(dt.naive_utc()));
        }

        // Try parsing as basic ISO format without timezone, assume UTC.
        if let Ok(dt) = NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S") {
            return Ok(Self(dt));
        }

        // Parse ISO format with microseconds but no timezone, assume UTC.
        if let Ok(dt) = NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S%.f") {
            return Ok(Self(dt));
        }

        Err(InputValueError::custom(format!("invalid datetime: {s}")))
    }

    /// Convert NaiveDateTime to Timestamp and use its RFC3339 formatting.
    fn to_value(&self) -> Value {
        let s = Timestamp::from(self.0).to_rfc3339_string();

        Value::String(s)
    }
}

impl From<NaiveDateTime> for Iso8601DateTime {
    fn from(dt: NaiveDateTime) -> Self {
        Self(dt)
    }
}

impl From<Iso8601DateTime> for NaiveDateTime {
    fn from(dt: Iso8601DateTime) -> Self {
        dt.0
    }
}
