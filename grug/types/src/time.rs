use {
    crate::Inner,
    borsh::{BorshDeserialize, BorshSerialize},
    grug_math::{Dec, Int, IsZero, NumberConst, Udec128_9, Uint128},
    serde::{Deserialize, Serialize},
    std::ops::{Add, Mul, Sub},
};

/// The number of nanoseconds in a microsecond.
const NANOS_PER_MICRO: u128 = 1_000;
/// The number of microseconds in a millisecond.
const MICROS_PER_MILLI: u128 = 1_000;
/// The number of milliseconds in a second.
const MILLIS_PER_SECOND: u128 = 1_000;
/// The number of seconds in a minute.
const SECONDS_PER_MINUTE: u128 = 60;
/// The number of minutes in an hour.
const MINUTES_PER_HOUR: u128 = 60;
/// The number of hours in a day.
const HOURS_PER_DAY: u128 = 24;
/// The number of days in a week.
const DAYS_PER_WEEK: u128 = 7;

/// UNIX epoch timestamp, in nanosecond precision.
///
/// A timestamp is simply a duration between a point of time and the UNIX epoch,
/// so here we define timestamp simply as an alias to [`Duration`](crate::Duration).
pub type Timestamp = Duration;

/// A span of time, in nanosecond precision.
///
/// We can't use [`std::time::Duration`](std::time::Duration) because it doesn't
/// implement the Borsh traits. Additionally, it's serialized to JSON as a
/// struct, e.g. `{"seconds":123,"nanos":123}`, which isn't desirable.
#[derive(
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
    Default,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
pub struct Duration(Dec<u128, 9>);

impl Duration {
    pub const ZERO: Self = Self(Dec::ZERO);

    pub const fn from_nanos(nanos: u128) -> Self {
        Self(Dec::raw(Int::new(nanos)))
    }

    pub const fn from_micros(micros: u128) -> Self {
        Self::from_nanos(micros * NANOS_PER_MICRO)
    }

    pub const fn from_millis(millis: u128) -> Self {
        Self::from_micros(millis * MICROS_PER_MILLI)
    }

    pub const fn from_seconds(seconds: u128) -> Self {
        Self::from_millis(seconds * MILLIS_PER_SECOND)
    }

    pub const fn from_minutes(minutes: u128) -> Self {
        Self::from_seconds(minutes * SECONDS_PER_MINUTE)
    }

    pub const fn from_hours(hours: u128) -> Self {
        Self::from_minutes(hours * MINUTES_PER_HOUR)
    }

    pub const fn from_days(days: u128) -> Self {
        Self::from_hours(days * HOURS_PER_DAY)
    }

    pub const fn from_weeks(weeks: u128) -> Self {
        Self::from_days(weeks * DAYS_PER_WEEK)
    }

    pub fn into_nanos(self) -> u128 {
        self.0.into_inner()
    }

    pub fn into_micros(self) -> u128 {
        self.into_nanos() / NANOS_PER_MICRO
    }

    pub fn into_millis(self) -> u128 {
        self.into_micros() / MICROS_PER_MILLI
    }

    pub fn into_seconds(self) -> u128 {
        self.into_millis() / MILLIS_PER_SECOND
    }

    pub fn into_minutes(self) -> u128 {
        self.into_seconds() / SECONDS_PER_MINUTE
    }

    pub fn into_hours(self) -> u128 {
        self.into_minutes() / MINUTES_PER_HOUR
    }

    pub fn into_days(self) -> u128 {
        self.into_hours() / HOURS_PER_DAY
    }

    pub fn into_weeks(self) -> u128 {
        self.into_days() / DAYS_PER_WEEK
    }
}

#[cfg(feature = "chrono")]
impl Timestamp {
    pub fn to_utc_date_time(&self) -> chrono::DateTime<chrono::Utc> {
        // This panics if the timestamp (as nanoseconds) overflows `i64` range.
        // But that'd be 500 years or so from now...
        chrono::DateTime::from_timestamp_nanos(self.into_nanos() as i64)
    }

    pub fn to_naive_date_time(&self) -> chrono::NaiveDateTime {
        self.to_utc_date_time().naive_utc()
    }

    pub fn to_rfc3339_string(&self) -> String {
        self.to_utc_date_time().to_rfc3339()
    }
}

impl Inner for Duration {
    type U = Udec128_9;

    fn inner(&self) -> &Self::U {
        &self.0
    }

    fn into_inner(self) -> Self::U {
        self.0
    }
}

impl IsZero for Duration {
    fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
}

impl Add for Duration {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub for Duration {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl Mul for Duration {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}

impl<U> Mul<U> for Duration
where
    U: Into<Uint128>,
{
    type Output = Self;

    fn mul(self, rhs: U) -> Self::Output {
        Self(self.0 * Dec::<u128, 9>::new(rhs.into().into_inner()))
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use crate::{BorshDeExt, BorshSerExt, JsonDeExt, JsonSerExt, ResultExt, Timestamp};

    #[test]
    fn serialization_works() {
        const TIMESTAMP: Timestamp = Timestamp::from_nanos(1732770602144737024);
        const TIMESTAMP_JSON: &str = "\"1732770602.144737024\"";

        // json
        TIMESTAMP
            .to_json_string()
            .should_succeed_and_equal(TIMESTAMP_JSON)
            .deserialize_json::<Timestamp>()
            .should_succeed_and_equal(TIMESTAMP);

        // borsh
        TIMESTAMP
            .to_borsh_vec()
            .should_succeed()
            .deserialize_borsh::<Timestamp>()
            .should_succeed_and_equal(TIMESTAMP);
    }
}
