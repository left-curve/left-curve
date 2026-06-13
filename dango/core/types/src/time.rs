#[cfg(feature = "chrono")]
use chrono::{DateTime, NaiveDateTime, SecondsFormat, Utc};
use {
    crate::{Inner, NonZero},
    borsh::{BorshDeserialize, BorshSerialize},
    dango_math::{Dec, Int, IsZero, MathResult, Number, NumberConst, Uint128},
    serde::{Deserialize, Serialize},
    std::ops::{Add, Div, Mul, Rem, Sub},
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
    pub const MAX: Self = Self(Dec::MAX);
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

    /// Convert the `grug::Duration` to a `std::time::Duration`.
    ///
    /// ## Panics
    ///
    /// Panics when the duration, expressed in nanoseconds, overflows the `u64`
    /// range. This event will happen at 2554-07-21 23:34:33.709551615 UTC.
    pub fn into_std(self) -> std::time::Duration {
        std::time::Duration::from_nanos(self.into_nanos() as u64)
    }

    /// Truncate down to the nearest multiple of `term`. For a [`Timestamp`]
    /// (UNIX-epoch aligned), this snaps to the start of the bucket of width
    /// `term`. `term` is enforced non-zero by the [`NonZero`] wrapper, so
    /// this method cannot divide by zero.
    pub fn truncate(self, term: NonZero<Duration>) -> Self {
        let nanos = self.into_nanos();
        let term_nanos = term.into_inner().into_nanos();
        Self::from_nanos(nanos - (nanos % term_nanos))
    }

    /// Truncate down to the start of the hour.
    pub fn truncate_to_hour(self) -> Self {
        self.truncate(NonZero::new_unchecked(Self::from_hours(1)))
    }

    /// Truncate down to the start of the day.
    pub fn truncate_to_day(self) -> Self {
        self.truncate(NonZero::new_unchecked(Self::from_days(1)))
    }
}

#[cfg(feature = "chrono")]
impl Timestamp {
    /// Convert the `grug::Timestamp` to a `chrono::DateTime<Utc>`.
    ///
    /// ## Panics
    ///
    /// Panics when the timestamp, expressed in nanoseconds, overflows the `i64`
    /// range. This event will happen at 2262-04-11 23:47:16.854775807 UTC.
    pub fn to_utc_date_time(self) -> DateTime<Utc> {
        DateTime::from_timestamp_nanos(self.into_nanos() as i64)
    }

    pub fn to_naive_date_time(self) -> NaiveDateTime {
        self.to_utc_date_time().naive_utc()
    }

    pub fn to_rfc3339_string(self) -> String {
        self.to_utc_date_time()
            .to_rfc3339_opts(SecondsFormat::Nanos, true)
    }
}

impl From<std::time::Duration> for Duration {
    fn from(duration: std::time::Duration) -> Self {
        Self::from_nanos(duration.as_nanos())
    }
}

impl From<Duration> for std::time::Duration {
    fn from(duration: Duration) -> Self {
        std::time::Duration::from_nanos(duration.into_nanos() as u64)
    }
}

#[cfg(feature = "chrono")]
impl From<DateTime<Utc>> for Timestamp {
    /// ## Panics
    ///
    /// This method panics after April 11, 2262 when the UNIX timestamp, denoted
    /// in nanoseconds, goes out of the `i64` range.
    fn from(datetime: DateTime<Utc>) -> Self {
        let Some(nanos) = datetime.timestamp_nanos_opt() else {
            panic!("UNIX timestamp is out of `i64` range: {datetime}");
        };

        Self::from_nanos(nanos as u128)
    }
}

#[cfg(feature = "chrono")]
impl From<NaiveDateTime> for Timestamp {
    fn from(datetime: NaiveDateTime) -> Self {
        datetime.and_utc().into()
    }
}

impl Inner for Duration {
    type U = Dec<u128, 9>;

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

impl Div for Duration {
    // Dividing a timestamp by another timestamp should yield a dimensionless
    // quantity. We show this by returning the inner type `Dec<u128, 9>`
    // (instead of another `Timestamp`) as output.
    type Output = <Self as Inner>::U;

    fn div(self, rhs: Self) -> Self::Output {
        self.0 / rhs.0
    }
}

impl Rem for Duration {
    type Output = Self;

    fn rem(self, rhs: Self) -> Self::Output {
        Self(self.0 % rhs.0)
    }
}

impl Number for Duration {
    fn checked_add(self, other: Self) -> MathResult<Self> {
        self.0.checked_add(other.0).map(Self)
    }

    fn checked_sub(self, other: Self) -> MathResult<Self> {
        self.0.checked_sub(other.0).map(Self)
    }

    fn checked_mul(self, other: Self) -> MathResult<Self> {
        self.0.checked_mul(other.0).map(Self)
    }

    fn checked_div(self, other: Self) -> MathResult<Self> {
        self.0.checked_div(other.0).map(Self)
    }

    fn checked_rem(self, other: Self) -> MathResult<Self> {
        self.0.checked_rem(other.0).map(Self)
    }

    fn saturating_add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }

    fn saturating_mul(self, other: Self) -> Self {
        Self(self.0.saturating_mul(other.0))
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use crate::{
        BorshDeExt, BorshSerExt, Duration, JsonDeExt, JsonSerExt, NonZero, ResultExt, Timestamp,
    };

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

    #[test]
    fn truncate_works() {
        let one_hour = NonZero::new(Duration::from_hours(1)).unwrap();
        let ninety_min = Duration::from_minutes(90);

        // 1h30m truncated by 1h is 1h.
        assert_eq!(ninety_min.truncate(one_hour), Duration::from_hours(1));

        // 1h30m truncated by 30min is 1h30m.
        assert_eq!(
            ninety_min.truncate(NonZero::new(Duration::from_minutes(30)).unwrap()),
            ninety_min,
        );

        // Zero truncates to zero.
        assert_eq!(Duration::ZERO.truncate(one_hour), Duration::ZERO);

        // `NonZero::new` rejects a zero term at construction time, so
        // `truncate` itself can never divide by zero.
        assert!(NonZero::new(Duration::ZERO).is_err());
    }

    #[test]
    fn truncate_to_hour_works() {
        // 3 days, 7 hours, 42 minutes, 17 seconds since the epoch.
        let ts = Duration::from_days(3)
            + Duration::from_hours(7)
            + Duration::from_minutes(42)
            + Duration::from_seconds(17);

        // Truncating to the hour drops the 42m 17s remainder, leaving exactly
        // 3 days + 7 hours.
        assert_eq!(
            ts.truncate_to_hour(),
            Duration::from_days(3) + Duration::from_hours(7),
        );
    }

    #[test]
    fn truncate_to_day_works() {
        // 5 days, 13 hours, 28 minutes, 9 seconds since the epoch.
        let ts = Duration::from_days(5)
            + Duration::from_hours(13)
            + Duration::from_minutes(28)
            + Duration::from_seconds(9);

        // Truncating to the day drops the 13h 28m 09s remainder.
        assert_eq!(ts.truncate_to_day(), Duration::from_days(5));
    }
}
