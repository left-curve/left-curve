use {
    crate::{Prefixer, PrimaryKey},
    grug_types::Bound,
};

// --------------------------------- raw bound ---------------------------------

/// Like Bound but only with the raw binary variants.
pub enum RawBound {
    Inclusive(Vec<u8>),
    Exclusive(Vec<u8>),
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
