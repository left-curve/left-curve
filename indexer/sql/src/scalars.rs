use {
    async_graphql::{Scalar, ScalarType, Value},
    grug_types::Timestamp,
    sqlx::types::chrono::{DateTime as ChronoDateTime, NaiveDateTime},
};

/// A custom DateTime scalar that ensures ISO8601 compliance with timezone information
#[derive(Clone, Debug)]
pub struct Iso8601DateTime(pub NaiveDateTime);

#[Scalar(name = "DateTime")]
impl ScalarType for Iso8601DateTime {
    fn parse(value: Value) -> async_graphql::InputValueResult<Self> {
        match value {
            Value::String(s) => {
                // Try parsing as RFC3339 first (with timezone)
                if let Ok(dt) = ChronoDateTime::parse_from_rfc3339(&s) {
                    Ok(Self(dt.naive_utc()))
                } else if let Ok(dt) = NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S") {
                    // Parse basic ISO format without timezone, assume UTC
                    Ok(Self(dt))
                } else if let Ok(dt) = NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S%.f") {
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
        // Convert NaiveDateTime to Timestamp and use its RFC3339 formatting
        let ts = Timestamp::from_nanos(self.0.and_utc().timestamp_nanos_opt().unwrap_or(0) as u128);
        Value::String(ts.to_rfc3339_string())
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
