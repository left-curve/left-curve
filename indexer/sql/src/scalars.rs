use {
    crate::serde_iso8601,
    async_graphql::{InputValueError, InputValueResult, Scalar, ScalarType, Value},
    grug_types::Timestamp,
    sqlx::types::chrono::NaiveDateTime,
};

/// A custom DateTime scalar that ensures ISO8601 compliance with timezone information
#[derive(Clone, Debug)]
pub struct Iso8601DateTime(pub NaiveDateTime);

#[Scalar(name = "DateTime")]
impl ScalarType for Iso8601DateTime {
    fn parse(value: Value) -> InputValueResult<Self> {
        serde_iso8601::deserialize(value)
            .map(Self)
            .map_err(InputValueError::custom)
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
