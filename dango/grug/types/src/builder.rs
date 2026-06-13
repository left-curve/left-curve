//! Types used in creating builder types with the _type state_ pattern, which
//! allows catching some user errors at compile time.
//!
//! See [this video](https://youtu.be/pwmIQzLuYl0) for context.

use {
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
    std::{fmt::Debug, marker::PhantomData},
};

/// Represents a builder parameter that has not yet been provided.
#[derive(
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
pub struct Undefined<T = ()>(PhantomData<T>);

impl<T> Undefined<T> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<T> Default for Undefined<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents a builder parameter that has already been provided.
#[derive(
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
pub struct Defined<T>(T);

impl<T> Defined<T> {
    pub fn new(inner: T) -> Self {
        Self(inner)
    }

    pub fn inner(&self) -> &T {
        &self.0
    }

    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.0
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

/// Represents a builder parameter that may or may not have been provided.
pub trait MaybeDefined<T> {
    fn maybe_inner(&self) -> Option<&T>;

    fn maybe_into_inner(self) -> Option<T>;
}

impl<T> MaybeDefined<T> for Defined<T> {
    fn maybe_inner(&self) -> Option<&T> {
        Some(self.inner())
    }

    fn maybe_into_inner(self) -> Option<T> {
        Some(self.into_inner())
    }
}

impl<T> MaybeDefined<T> for Undefined<T> {
    fn maybe_inner(&self) -> Option<&T> {
        None
    }

    fn maybe_into_inner(self) -> Option<T> {
        None
    }
}
