use {
    crate::{PrimaryKey, RawKey},
    grug_types::nested_namespaces_with_key,
};

/// Describes a prefix in a composite key.
///
/// This trait is different from [`PrimaryKey`](crate::PrimaryKey) in two way:
///
/// 1. `PrimaryKey::joined_key` prefixes each raw key except for the last one
///    with their lengths. However, `Prefixer::joined_prefix` prefixes all raw
///    key with lengths, _including_ the last one.
/// 2. `Prefixer` does not have a `from_slice` method because not needed.
pub trait Prefixer {
    fn raw_prefixes(&self) -> Vec<RawKey<'_>>;

    fn joined_prefix(&self) -> Vec<u8> {
        let raw_prefixes = self.raw_prefixes();
        nested_namespaces_with_key(None, &raw_prefixes, Option::<RawKey>::None)
    }
}

impl<T> Prefixer for T
where
    T: PrimaryKey,
{
    fn raw_prefixes(&self) -> Vec<RawKey<'_>> {
        self.raw_keys()
    }
}
