use {
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
};

/// Represents a number of bytes.
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
pub struct Size(usize);

impl Size {
    /// Create a size of `n` bytes.
    pub const fn new(n: usize) -> Self {
        Size(n)
    }

    /// Create a size of `n` kilobytes.
    pub const fn kilo(n: usize) -> Self {
        Size(n * 1000)
    }

    /// Create a size of `n` kibibytes.
    pub const fn kibi(n: usize) -> Self {
        Size(n * 1024)
    }

    /// Create a size of `n` megabytes.
    pub const fn mega(n: usize) -> Self {
        Size(n * 1000 * 1000)
    }

    /// Create a size of `n` mebibytes.
    pub const fn mebi(n: usize) -> Self {
        Size(n * 1024 * 1024)
    }

    /// Create a size of `n` gigabytes.
    pub const fn giga(n: usize) -> Self {
        Size(n * 1000 * 1000 * 1000)
    }

    /// Create a size of `n` gibibytes.
    pub const fn gibi(n: usize) -> Self {
        Size(n * 1024 * 1024 * 1024)
    }

    /// Consume self, return the number of bytes as a `usize`.
    pub const fn bytes(self) -> usize {
        self.0
    }
}
