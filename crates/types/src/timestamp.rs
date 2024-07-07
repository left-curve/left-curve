use {
    crate::Uint128,
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
};

const NANOS_PER_SECOND: u128 = 1_000_000_000;

/// UNIX epoch timestamp in nanosecond precision.
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
pub struct Timestamp(Uint128);

impl Timestamp {
    pub const fn from_nanos(nanos: u128) -> Self {
        Self(Uint128::new(nanos))
    }

    pub const fn from_seconds(seconds: u128) -> Self {
        Self(Uint128::new(seconds * NANOS_PER_SECOND))
    }

    pub fn plus_nanos(&self, nanos: u128) -> Self {
        Self(self.0 + Uint128::new(nanos))
    }

    pub fn plus_seconds(&self, seconds: u128) -> Self {
        self.plus_nanos(seconds * NANOS_PER_SECOND)
    }

    // TODO: add more plus/minus methods

    #[inline]
    pub fn nanos(&self) -> u128 {
        self.0.number()
    }

    #[inline]
    pub fn seconds(&self) -> u128 {
        self.0.number() / NANOS_PER_SECOND
    }

    #[inline]
    pub fn subsec_nanos(&self) -> u128 {
        self.0.number() % NANOS_PER_SECOND
    }
}
