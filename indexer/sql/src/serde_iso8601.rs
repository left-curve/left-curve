use {
    grug_types::Timestamp,
    sea_orm::entity::prelude::DateTime,
    serde::{Deserialize, Deserializer, Serializer},
    sqlx::types::chrono,
};

/// Serialize a NaiveDateTime as ISO8601 with timezone (UTC).
pub fn serialize<S>(date: &DateTime, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let s = Timestamp::from(*date).to_rfc3339_string();

    serializer.serialize_str(&s)
}

/// Deserialize an ISO8601 string to NaiveDateTime.
pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;

    // Try parsing as RFC3339 first (with timezone).
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&s) {
        return Ok(dt.naive_utc());
    }

    // Try parsing as basic ISO format without timezone, assume UTC.
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S") {
        return Ok(dt);
    }

    // Parse ISO format with microseconds but no timezone, assume UTC.
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S%.f") {
        return Ok(dt);
    }

    Err(serde::de::Error::custom(format!("invalid datetime: {s}")))
}
