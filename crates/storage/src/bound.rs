use crate::{Prefixer, PrimaryKey};

// --------------------------------- raw bound ---------------------------------

/// Like Bound but only with the raw binary variants.
pub enum RawBound {
    Inclusive(Vec<u8>),
    Exclusive(Vec<u8>),
}

// ----------------------------------- bound -----------------------------------

/// Describe the limit for iteration.
///
/// Typically we use an `Option<Bound<T>>` in contracts, where `T` implements
/// the `Key` trait.
///
/// Compared to `std::ops::Bound`, it removes the unbounded option (which is to
/// be represented by a `None`), and introduces the "raw" variants. We don't use
/// std `Bound` because it typically requires more verbose code in contracts.
pub enum Bound<K> {
    Inclusive(K),
    Exclusive(K),
}

impl<K> Bound<K> {
    pub fn inclusive<T>(t: T) -> Self
    where
        T: Into<K>,
    {
        Self::Inclusive(t.into())
    }

    pub fn exclusive<T>(t: T) -> Self
    where
        T: Into<K>,
    {
        Self::Exclusive(t.into())
    }
}

impl<K> From<Bound<K>> for RawBound
where
    K: PrimaryKey,
{
    fn from(bound: Bound<K>) -> Self {
        match bound {
            Bound::Inclusive(k) => RawBound::Inclusive(k.joined_key()),
            Bound::Exclusive(k) => RawBound::Exclusive(k.joined_key()),
        }
    }
}

// ------------------------------- prefix bound --------------------------------

pub enum PrefixBound<K>
where
    K: PrimaryKey,
{
    Inclusive(K::Prefix),
    Exclusive(K::Prefix),
}

impl<K> PrefixBound<K>
where
    K: PrimaryKey,
{
    pub fn inclusive<P>(p: P) -> Self
    where
        P: Into<K::Prefix>,
    {
        Self::Inclusive(p.into())
    }

    pub fn exclusive<P>(p: P) -> Self
    where
        P: Into<K::Prefix>,
    {
        Self::Exclusive(p.into())
    }
}

impl<K> From<PrefixBound<K>> for RawBound
where
    K: PrimaryKey,
{
    fn from(bound: PrefixBound<K>) -> Self {
        match bound {
            PrefixBound::Inclusive(p) => RawBound::Inclusive(p.joined_prefix()),
            PrefixBound::Exclusive(p) => RawBound::Exclusive(p.joined_prefix()),
        }
    }
}
