use {
    grug_types::Timestamp,
    sea_orm::entity::prelude::DateTime,
    serde::{Deserialize, Deserializer, Serializer},
    sqlx::types::chrono,
};

/// Serialize a NaiveDateTime as ISO8601 with timezone (UTC)
pub fn serialize<S>(date: &DateTime, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    // Convert NaiveDateTime to Timestamp for proper RFC3339 formatting
    let ts = Timestamp::from_nanos(date.and_utc().timestamp_nanos_opt().unwrap_or(0) as u128);
    serializer.serialize_str(&ts.to_rfc3339_string())
}

/// Deserialize an ISO8601 string to NaiveDateTime
pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;

    // Try parsing as RFC3339 first (with timezone)
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&s) {
        Ok(dt.naive_utc())
    } else if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S") {
        // Parse basic ISO format without timezone, assume UTC
        Ok(dt)
    } else if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S%.f") {
        // Parse ISO format with microseconds but no timezone, assume UTC
        Ok(dt)
    } else {
        Err(serde::de::Error::custom("Invalid datetime format"))
    }
}
