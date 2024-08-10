use {
    grug_types::{
        nested_namespaces_with_key, Addr, Bytable, Duration, Hash, StdError, StdResult, Uint128,
        Uint256, Uint512, Uint64,
    },
    std::{borrow::Cow, mem},
};

// ------------------------------------ key ------------------------------------

/// Describes a key used in mapping data structures, i.e. [`Map`](crate::Map)
/// and [`IndexedMap`](crate::IndexedMap).
///
/// The key needs to be serialized to or deserialized from raw bytes. However,
/// we don't want to use `serde` here because it's slow, not compact, and
/// faillable.
///
/// Additionally, compound keys can be split into `Prefix` and `Suffix`, which
/// are useful in iterations.
pub trait Key {
    /// The number of elements in a tuple key.
    ///
    /// E.g.,
    ///
    /// - for singleton keys, this is 1 (the default value).
    /// - for `(A, B)`, this is 2;
    /// - for `(A, B, C)`, this is 3;
    ///
    /// so on.
    ///
    /// This value is necessary for deserializing _nested_ tuple keys.
    ///
    /// For example, consider the following nested tuple key: `((A, B), (C, D))`.
    /// This key is serialized into the following bytes:
    ///
    /// ```plain
    /// len(A) | A | len(B) | B | len(C) | C | D
    /// ```
    ///
    /// Without knowing the number of key elements, we don't know how to
    /// deserialize this: whether it's `((A, B), (C, D))`, or `((A, B, C), (D))`,
    /// or else?
    ///
    /// Only if we know each element in the tuple themselves each has two
    /// elements, can we deserialize this correctly.
    ///
    /// See the following PR for details: <https://github.com/CosmWasm/cw-storage-plus/pull/34>.
    const KEY_ELEMS: u16 = 1;

    /// For tuple keys, the first element.
    ///
    /// E.g. for `(A, B)`, this is `A`.
    ///
    /// Use `()` for singleton keys.
    ///
    /// This is used for iterations. E.g. given a value of `A`, we can iterate
    /// all values of `B` in the map.
    type Prefix: Prefixer;

    /// For tuple keys, the elements _excluding_ the `Prefix`.
    ///
    /// E.g. for `(A, B)`, this is `B`.
    ///
    /// Use `()` for singleton keys.
    type Suffix;

    /// The type that raw keys deserialize into, which may be different from the
    /// key itself.
    ///
    /// E.g. when `&str` is used as the key, it deserializes into `String`.
    type Output;

    /// Convert the key into one or more _raw keys_. Each raw key is a byte slice,
    /// either owned or a reference, represented as a `Cow<[u8]>`.
    fn raw_keys(&self) -> Vec<Cow<[u8]>>;

    /// Serialize the raw keys into bytes.
    ///
    /// Each raw key, other than the last one, is prefixed by its length. This
    /// is such that when deserializing, we can tell where a raw key ends and
    /// where the next one starts.
    ///
    /// For example, if the raw keys are `vec![A, B, C, D]`, they are serialized
    /// into:
    ///
    /// ```plain
    /// len(A) | A | len(B) | B | len(C) | C | D
    /// ```
    ///
    /// where `len()` denotes the length, as a 16-bit big endian number;
    /// `|` denotes byte concatenation.
    fn joined_key(&self) -> Vec<u8> {
        let mut raw_keys = self.raw_keys();
        let last_raw_key = raw_keys.pop();
        nested_namespaces_with_key(None, &raw_keys, last_raw_key.as_ref())
    }

    /// Deserialize the raw bytes into the output.
    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output>;
}

impl Key for () {
    type Output = ();
    type Prefix = ();
    type Suffix = ();

    fn raw_keys(&self) -> Vec<Cow<[u8]>> {
        vec![]
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        if !bytes.is_empty() {
            return Err(StdError::deserialize::<Self::Output, _>(
                "key",
                "expecting empty bytes",
            ));
        }

        Ok(())
    }
}

impl Key for &[u8] {
    type Output = Vec<u8>;
    type Prefix = ();
    type Suffix = ();

    fn raw_keys(&self) -> Vec<Cow<[u8]>> {
        vec![Cow::Borrowed(self)]
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        Ok(bytes.to_vec())
    }
}

impl Key for Vec<u8> {
    type Output = Vec<u8>;
    type Prefix = ();
    type Suffix = ();

    fn raw_keys(&self) -> Vec<Cow<[u8]>> {
        vec![Cow::Borrowed(self)]
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        Ok(bytes.to_vec())
    }
}

impl Key for &str {
    type Output = String;
    type Prefix = ();
    type Suffix = ();

    fn raw_keys(&self) -> Vec<Cow<[u8]>> {
        vec![Cow::Borrowed(self.as_bytes())]
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        String::from_utf8(bytes.to_vec())
            .map_err(|err| StdError::deserialize::<Self::Output, _>("key", err))
    }
}

impl Key for String {
    type Output = String;
    type Prefix = ();
    type Suffix = ();

    fn raw_keys(&self) -> Vec<Cow<[u8]>> {
        vec![Cow::Borrowed(self.as_bytes())]
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        String::from_utf8(bytes.to_vec())
            .map_err(|err| StdError::deserialize::<Self::Output, _>("key", err))
    }
}

impl Key for Addr {
    type Output = Addr;
    type Prefix = ();
    type Suffix = ();

    fn raw_keys(&self) -> Vec<Cow<[u8]>> {
        vec![Cow::Borrowed(self.as_ref())]
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        bytes.try_into()
    }
}

impl<const N: usize> Key for Hash<N> {
    type Output = Hash<N>;
    type Prefix = ();
    type Suffix = ();

    fn raw_keys(&self) -> Vec<Cow<[u8]>> {
        vec![Cow::Borrowed(self.as_ref())]
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        bytes.try_into()
    }
}

impl Key for Duration {
    type Output = Duration;
    type Prefix = ();
    type Suffix = ();

    fn raw_keys(&self) -> Vec<Cow<[u8]>> {
        vec![Cow::Owned(self.into_nanos().to_be_bytes().to_vec())]
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        let nanos = u128::from_be_bytes(bytes.try_into()?);
        Ok(Duration::from_nanos(nanos))
    }
}

impl<K> Key for &K
where
    K: Key,
{
    type Output = K::Output;
    type Prefix = K::Prefix;
    type Suffix = K::Suffix;

    fn raw_keys(&self) -> Vec<Cow<[u8]>> {
        (*self).raw_keys()
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        K::from_slice(bytes)
    }
}

impl<A, B> Key for (A, B)
where
    A: Key + Prefixer,
    B: Key,
{
    type Output = (A::Output, B::Output);
    type Prefix = A;
    type Suffix = B;

    const KEY_ELEMS: u16 = A::KEY_ELEMS + B::KEY_ELEMS;

    fn raw_keys(&self) -> Vec<Cow<[u8]>> {
        let mut keys = self.0.raw_keys();
        keys.extend(self.1.raw_keys());
        keys
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        let (a_raw, b_raw) = split_first_key(A::KEY_ELEMS, bytes);

        let a = A::from_slice(&a_raw)?;
        let b = B::from_slice(b_raw)?;

        Ok((a, b))
    }
}

impl<A, B, C> Key for (A, B, C)
where
    A: Key + Prefixer,
    B: Key,
    C: Key,
{
    type Output = (A::Output, B::Output, C::Output);
    // Here we make `A` as the prefix and `(B, C)` as the suffix.
    //
    // This means you can give a value of `A` and iterate all values of `B` and `C`.
    //
    // If you'd like to give a value of `(A, B)` and iterate all values of `C`,
    // use this syntax:
    //
    // ```pseudocode
    // MAP.prefix(A).append(B).range(...);
    // ```
    type Prefix = A;
    type Suffix = (B, C);

    const KEY_ELEMS: u16 = A::KEY_ELEMS + B::KEY_ELEMS + C::KEY_ELEMS;

    fn raw_keys(&self) -> Vec<Cow<[u8]>> {
        let mut keys = self.0.raw_keys();
        keys.extend(self.1.raw_keys());
        keys.extend(self.2.raw_keys());
        keys
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        let (a_raw, bc_raw) = split_first_key(A::KEY_ELEMS, bytes);
        let (b_raw, c_raw) = split_first_key(B::KEY_ELEMS, bc_raw);

        let a = A::from_slice(&a_raw)?;
        let b = B::from_slice(&b_raw)?;
        let c = C::from_slice(c_raw)?;

        Ok((a, b, c))
    }
}

/// Given the raw bytes of a tuple key consisting of at least one subkey, each
/// subkey having one or more key elements, split off the first subkey.
///
/// E.g. consider the tuple key `((A, B, C), (D, E))`:
///
/// - `(A, B, C)` is the first subkey; it has `KEY_ELEMS` of 3.
/// - `(D, E)` is the second subkey; it has `KEY_ELEMS` of 2.
///
/// This tuple key is serialized as:
///
/// ```plain
/// len(A) | A | len(B) | B | len(C) | C | len(D) | D | E
/// ```
///
/// We want to split off the first subkey as:
///
/// ```plain
/// len(A) | A | len(B) | B | C
/// ```
///
/// Note that the last element `C` does not have its length prefix, while the
/// other elements retain their length prefixes.
///
/// The remaining byte slice:
///
/// ```plain
/// len(D) | D | E
/// ```
///
/// is also returned.
pub(crate) fn split_first_key(key_elems: u16, value: &[u8]) -> (Vec<u8>, &[u8]) {
    let mut index = 0;
    let mut first_key = Vec::new();

    for i in 0..key_elems {
        let len_slice = &value[index..index + 2];
        index += 2;

        // Elements other than the last one retain their length prefixes.
        if i < key_elems - 1 {
            first_key.extend_from_slice(len_slice);
        }

        let elem_len = u16::from_be_bytes(len_slice.try_into().unwrap()) as usize;
        first_key.extend_from_slice(&value[index..index + elem_len]);
        index += elem_len;
    }

    let remainder = &value[index..];

    (first_key, remainder)
}

macro_rules! impl_integer_key {
    ($($t:ty),+ $(,)?) => {
        $(impl Key for $t {
            type Prefix = ();
            type Suffix = ();
            type Output = $t;

            fn raw_keys(&self) -> Vec<Cow<[u8]>> {
                vec![Cow::Owned(self.to_be_bytes().to_vec())]
            }

            fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
                let Ok(bytes) = <[u8; mem::size_of::<Self>()]>::try_from(bytes) else {
                    return Err(StdError::deserialize::<Self::Output, _>(
                        "key",
                        format!(
                            "wrong number of bytes: expecting {}, got {}",
                            mem::size_of::<Self>(),
                            bytes.len(),
                        ),
                    ));
                };

                Ok(Self::from_be_bytes(bytes))
            }
        })*
    }
}

impl_integer_key!(
    u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, Uint64, Uint128, Uint256, Uint512,
);

// --------------------------------- prefixer ----------------------------------

pub trait Prefixer {
    fn raw_prefixes(&self) -> Vec<Cow<[u8]>>;

    fn joined_prefix(&self) -> Vec<u8> {
        let raw_prefixes = self.raw_prefixes();
        nested_namespaces_with_key(None, &raw_prefixes, None)
    }
}

impl Prefixer for () {
    fn raw_prefixes(&self) -> Vec<Cow<[u8]>> {
        vec![]
    }
}

impl Prefixer for &[u8] {
    fn raw_prefixes(&self) -> Vec<Cow<[u8]>> {
        vec![Cow::Borrowed(self)]
    }
}

impl Prefixer for Vec<u8> {
    fn raw_prefixes(&self) -> Vec<Cow<[u8]>> {
        vec![Cow::Borrowed(self)]
    }
}

impl Prefixer for &str {
    fn raw_prefixes(&self) -> Vec<Cow<[u8]>> {
        vec![Cow::Borrowed(self.as_bytes())]
    }
}

impl Prefixer for String {
    fn raw_prefixes(&self) -> Vec<Cow<[u8]>> {
        vec![Cow::Borrowed(self.as_bytes())]
    }
}

impl Prefixer for Addr {
    fn raw_prefixes(&self) -> Vec<Cow<[u8]>> {
        vec![Cow::Borrowed(self.as_ref())]
    }
}

impl<const N: usize> Prefixer for Hash<N> {
    fn raw_prefixes(&self) -> Vec<Cow<[u8]>> {
        vec![Cow::Borrowed(self.as_ref())]
    }
}

impl Prefixer for Duration {
    fn raw_prefixes(&self) -> Vec<Cow<[u8]>> {
        vec![Cow::Owned(self.into_nanos().to_be_bytes().to_vec())]
    }
}

impl<P> Prefixer for &P
where
    P: Prefixer,
{
    fn raw_prefixes(&self) -> Vec<Cow<[u8]>> {
        (*self).raw_prefixes()
    }
}

impl<A, B> Prefixer for (A, B)
where
    A: Prefixer,
    B: Prefixer,
{
    fn raw_prefixes(&self) -> Vec<Cow<[u8]>> {
        let mut prefixes = self.0.raw_prefixes();
        prefixes.extend(self.1.raw_prefixes());
        prefixes
    }
}

macro_rules! impl_integer_prefixer {
    ($($t:ty),+ $(,)?) => {
        $(impl Prefixer for $t {
            fn raw_prefixes(&self) -> Vec<Cow<[u8]>> {
                vec![Cow::Owned(self.to_be_bytes().to_vec())]
            }
        })*
    }
}

impl_integer_prefixer!(
    u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, Uint64, Uint128, Uint256, Uint512,
);

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triple_tuple_key() {
        type TripleTuple<'a> = (&'a str, &'a str, &'a str);

        let (a, b, c) = ("larry", "jake", "pumpkin");
        let serialized = (a, b, c).joined_key();
        let deserialized = TripleTuple::from_slice(&serialized).unwrap();

        assert_eq!(deserialized, (a.to_string(), b.to_string(), c.to_string()));
    }

    #[test]
    #[rustfmt::skip]
    fn nested_tuple_key() {
        // Two layers of nesting
        type NestedTuple<'a> = ((&'a str, &'a str), (&'a str, &'a str));

        let ((a, b), (c, d)) = (("larry", "engineer"), ("jake", "shepherd"));

        let serialized = ((a, b), (c, d)).joined_key();
        assert_eq!(serialized, [
            0, 5,                                   // len("larry")
            108, 97, 114, 114, 121,                 // "larry"
            0, 8,                                   // len("engineer")
            101, 110, 103, 105, 110, 101, 101, 114, // "engineer"
            0, 4,                                   // len("jake")
            106, 97, 107, 101,                      // "jake"
            115, 104, 101, 112, 104, 101, 114, 100, // "shepherd"
        ]);

        let deserialized = NestedTuple::from_slice(&serialized).unwrap();
        assert_eq!(
            deserialized,
            ((a.to_string(), b.to_string()), (c.to_string(), d.to_string()))
        );
    }

    #[test]
    #[rustfmt::skip]
    fn multi_nested_tuple_key() {
        // Three layers of nesting
        type NestedTuple<'a> = ((u64, (&'a str, &'a str)), &'a str);

        let ((a, (b, c)), d) = ((88888u64, ("larry", "engineer")), "jake");

        let serialized = ((a, (b, c)), d).joined_key();
        assert_eq!(serialized, [
            0, 8,                                   // len(u64)
            0, 0, 0, 0, 0, 1, 91, 56,               // 88888 = 1 * 256^2 + 91 * 256^1 + 56 * 256^0
            0, 5,                                   // len("larry")
            108, 97, 114, 114, 121,                 // "larry"
            0, 8,                                   // len("engineer")
            101, 110, 103, 105, 110, 101, 101, 114, // "engineer"
            106, 97, 107, 101,                      // "jake"
        ]);

        let deserialized = NestedTuple::from_slice(&serialized).unwrap();
        assert_eq!(
            deserialized,
            ((a, (b.to_string(), c.to_string())), d.to_string())
        );
    }
}
