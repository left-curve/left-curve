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

    deserialize_date_time(&s).map_err(serde::de::Error::custom)
}

pub fn deserialize_date_time(s: &str) -> Result<DateTime, String> {
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

    Err(format!("invalid datetime format: {s}"))
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, test_case::test_case};

    #[test_case(
        "2023-01-01T12:00:00Z" => Some(1672574400);
        "RFC 3339 with Z timezone"
    )]
    #[test_case(
        "2023-01-01T12:00:00+00:00" => Some(1672574400);
        "RFC 3339 with +00:00 timezone"
    )]
    #[test_case(
        "2023-01-01T12:00:00-05:00" => Some(1672592400);
        "RFC 3339 with negative timezone offset"
    )]
    #[test_case("2023-01-01T12:00:00" => Some(1672574400);
        "ISO format without timezone"
    )]
    #[test_case(
        "2023-01-01T12:00:00.123456" => Some(1672574400);
        "ISO format with microseconds"
    )]
    #[test_case(
        "2023-01-01T12:00:00.123" => Some(1672574400);
        "ISO format with milliseconds"
    )]
    #[test_case(
        "invalid-date" => None;
        "invalid date format"
    )]
    #[test_case(
        "2023-13-01T12:00:00Z" => None;
        "invalid month"
    )]
    #[test_case(
        "2023-01-01T25:00:00Z" => None;
        "invalid hour"
    )]
    #[test_case(
        "2023-01-01T12:60:00Z" => None;
        "invalid minute"
    )]
    #[test_case(
        "2023-01-01T12:00:61Z" => None;
        "invalid second"
    )]
    #[test_case(
        "not-a-date-at-all" => None;
        "completely invalid string"
    )]
    fn deserializing_date_time(s: &str) -> Option<i64> {
        deserialize_date_time(s)
            .map(|datetime| datetime.and_utc().timestamp())
            .ok()
    }
}
