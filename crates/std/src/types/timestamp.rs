use {
    crate::Uint64,
    serde::{Deserialize, Serialize},
};

const NANOS_PER_SECOND: u64 = 1_000_000_000;

/// UNIX epoch timestamp in nanosecond precision.
#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(Uint64);

impl Timestamp {
    pub const fn from_nanos(nanos: u64) -> Self {
        Self(Uint64::new(nanos))
    }

    pub const fn from_seconds(seconds: u64) -> Self {
        Self(Uint64::new(seconds * NANOS_PER_SECOND))
    }

    #[inline]
    pub fn nanos(&self) -> u64 {
        self.0.u64()
    }

    #[inline]
    pub fn seconds(&self) -> u64 {
        self.0.u64() / NANOS_PER_SECOND
    }

    #[inline]
    pub fn subsec_nanos(&self) -> u64 {
        self.0.u64() % NANOS_PER_SECOND
    }
}
