use {
    borsh::{BorshDeserialize, BorshSerialize},
    grug_math::{Inner, IsZero, Uint128},
    serde::{Deserialize, Serialize},
    std::ops::{Add, Mul, Sub},
};

/// How many nanoseconds are there in a second.
const NANOS_PER_SECOND: u128 = 1_000_000_000;
/// How many nanoseconds are there in a millisecond.
const NANOS_PER_MILLI: u128 = 1_000_000;
/// How many nenoseconds are there in a microsecond.
const NANOS_PER_MICRO: u128 = 1_000;

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
pub struct Duration(Uint128);

impl Duration {
    pub const fn from_seconds(seconds: u128) -> Self {
        Self(Uint128::new(seconds * NANOS_PER_SECOND))
    }

    pub const fn from_millis(millis: u128) -> Self {
        Self(Uint128::new(millis * NANOS_PER_MILLI))
    }

    pub const fn from_micros(micros: u128) -> Self {
        Self(Uint128::new(micros * NANOS_PER_MICRO))
    }

    pub const fn from_nanos(nanos: u128) -> Self {
        Self(Uint128::new(nanos))
    }

    pub fn into_seconds(self) -> u128 {
        self.0.into_inner() / NANOS_PER_SECOND
    }

    pub fn into_millis(self) -> u128 {
        self.0.into_inner() / NANOS_PER_MILLI
    }

    pub fn into_micros(self) -> u128 {
        self.0.into_inner() / NANOS_PER_MICRO
    }

    pub fn into_nanos(self) -> u128 {
        self.0.into_inner()
    }
}

impl Inner for Duration {
    type U = Uint128;

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

impl<U> Mul<U> for Duration
where
    U: Into<Uint128>,
{
    type Output = Self;

    fn mul(self, rhs: U) -> Self::Output {
        Self(self.0 * rhs.into())
    }
}
