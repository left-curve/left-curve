use {
    borsh::{BorshDeserialize, BorshSerialize},
    grug_math::{MathResult, Number, Uint128},
    serde::{Deserialize, Serialize},
    std::ops::{Add, Sub},
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
        self.0.number() / NANOS_PER_SECOND
    }

    pub fn into_millis(self) -> u128 {
        self.0.number() / NANOS_PER_MILLI
    }

    pub fn into_micros(self) -> u128 {
        self.0.number() / NANOS_PER_MICRO
    }

    pub fn into_nanos(self) -> u128 {
        self.0.number()
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

impl Number for Duration {
    fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

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

    fn checked_pow(self, other: u32) -> MathResult<Self> {
        self.0.checked_pow(other).map(Self)
    }

    fn checked_sqrt(self) -> MathResult<Self> {
        self.0.checked_sqrt().map(Self)
    }

    fn wrapping_add(self, other: Self) -> Self {
        Self(self.0.wrapping_add(other.0))
    }

    fn wrapping_sub(self, other: Self) -> Self {
        Self(self.0.wrapping_sub(other.0))
    }

    fn wrapping_mul(self, other: Self) -> Self {
        Self(self.0.wrapping_mul(other.0))
    }

    fn wrapping_pow(self, other: u32) -> Self {
        Self(self.0.wrapping_pow(other))
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

    fn saturating_pow(self, other: u32) -> Self {
        Self(self.0.saturating_pow(other))
    }
}
