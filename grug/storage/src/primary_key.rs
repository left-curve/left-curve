use {
    crate::{Prefixer, RawKey},
    bnum::{
        cast::CastFrom,
        types::{I256, I512, U256, U512},
    },
    grug_math::{Bytable, Dec, Int},
    grug_types::{
        Bounded, Bounds, CodeStatus, Denom, Duration, EncodedBytes, Encoder, Inner, LengthBounded,
        Lengthy, Part, StdError, StdResult, nested_namespaces_with_key,
    },
    std::{mem, str, vec},
};

/// Describes a key used in mapping data structures, i.e. [`Map`](crate::Map)
/// and [`IndexedMap`](crate::IndexedMap).
///
/// The key needs to be serialized to or deserialized from raw bytes. However,
/// we don't want to use `serde` here because it's slow, not compact, and
/// faillable.
///
/// Additionally, compound keys can be split into `Prefix` and `Suffix`, which
/// are useful in iterations.
pub trait PrimaryKey {
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
    const KEY_ELEMS: u8;

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

    /// Convert the key into one or more _raw keys_.
    fn raw_keys(&self) -> Vec<RawKey>;

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
        nested_namespaces_with_key(None, &raw_keys, last_raw_key)
    }

    /// Deserialize the raw bytes into the output.
    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output>;
}

impl PrimaryKey for () {
    type Output = ();
    type Prefix = ();
    type Suffix = ();

    const KEY_ELEMS: u8 = 1;

    fn raw_keys(&self) -> Vec<RawKey> {
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

impl PrimaryKey for bool {
    type Output = bool;
    type Prefix = ();
    type Suffix = ();

    const KEY_ELEMS: u8 = 1;

    fn raw_keys(&self) -> Vec<RawKey> {
        match self {
            false => vec![RawKey::Fixed8([0])],
            true => vec![RawKey::Fixed8([1])],
        }
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        match bytes {
            [0] => Ok(false),
            [1] => Ok(true),
            _ => Err(StdError::deserialize::<Self::Output, _>(
                "key",
                format!("unknown bytes `{bytes:?}` for boolean key, expecting 0 or 1"),
            )),
        }
    }
}

impl PrimaryKey for &[u8] {
    type Output = Vec<u8>;
    type Prefix = ();
    type Suffix = ();

    const KEY_ELEMS: u8 = 1;

    fn raw_keys(&self) -> Vec<RawKey> {
        vec![RawKey::Borrowed(self)]
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        Ok(bytes.to_vec())
    }
}

impl PrimaryKey for Vec<u8> {
    type Output = Vec<u8>;
    type Prefix = ();
    type Suffix = ();

    const KEY_ELEMS: u8 = 1;

    fn raw_keys(&self) -> Vec<RawKey> {
        vec![RawKey::Borrowed(self)]
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        Ok(bytes.to_vec())
    }
}

impl PrimaryKey for &str {
    type Output = String;
    type Prefix = ();
    type Suffix = ();

    const KEY_ELEMS: u8 = 1;

    fn raw_keys(&self) -> Vec<RawKey> {
        vec![RawKey::Borrowed(self.as_bytes())]
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        String::from_utf8(bytes.to_vec())
            .map_err(|err| StdError::deserialize::<Self::Output, _>("key", err))
    }
}

impl PrimaryKey for String {
    type Output = String;
    type Prefix = ();
    type Suffix = ();

    const KEY_ELEMS: u8 = 1;

    fn raw_keys(&self) -> Vec<RawKey> {
        vec![RawKey::Borrowed(self.as_bytes())]
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        String::from_utf8(bytes.to_vec())
            .map_err(|err| StdError::deserialize::<Self::Output, _>("key", err))
    }
}

impl<const S: usize, E> PrimaryKey for EncodedBytes<[u8; S], E>
where
    E: Encoder,
{
    type Output = EncodedBytes<[u8; S], E>;
    type Prefix = ();
    type Suffix = ();

    const KEY_ELEMS: u8 = 1;

    fn raw_keys(&self) -> Vec<RawKey> {
        vec![RawKey::Borrowed(self.as_ref())]
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        let inner = bytes.try_into()?;
        Ok(EncodedBytes::<[u8; S], E>::from_inner(inner))
    }
}

impl PrimaryKey for Part {
    type Output = Part;
    type Prefix = ();
    type Suffix = ();

    const KEY_ELEMS: u8 = 1;

    fn raw_keys(&self) -> Vec<RawKey> {
        vec![RawKey::Borrowed(self.as_bytes())]
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        str::from_utf8(bytes)
            .map_err(|err| StdError::deserialize::<Self::Output, _>("key", err))
            .and_then(TryInto::try_into)
    }
}

impl PrimaryKey for Denom {
    type Output = Denom;
    type Prefix = ();
    type Suffix = ();

    const KEY_ELEMS: u8 = 1;

    fn raw_keys(&self) -> Vec<RawKey> {
        vec![RawKey::Owned(self.to_string().into_bytes())]
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        String::from_utf8(bytes.to_vec())
            .map_err(|err| StdError::deserialize::<Self::Output, _>("key", err))
            .and_then(TryInto::try_into)
    }
}

impl PrimaryKey for Duration {
    type Output = Duration;
    type Prefix = ();
    type Suffix = ();

    const KEY_ELEMS: u8 = 1;

    fn raw_keys(&self) -> Vec<RawKey> {
        vec![RawKey::Fixed128(self.into_nanos().to_be_bytes())]
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        let nanos = u128::from_be_bytes(bytes.try_into()?);
        Ok(Duration::from_nanos(nanos))
    }
}

impl PrimaryKey for CodeStatus {
    type Output = CodeStatus;
    type Prefix = ();
    type Suffix = ();

    const KEY_ELEMS: u8 = 2;

    fn raw_keys(&self) -> Vec<RawKey> {
        match self {
            CodeStatus::Orphaned { since } => {
                vec![
                    RawKey::Fixed8([0]),
                    RawKey::Owned(since.into_nanos().to_be_bytes().to_vec()),
                ]
            },
            CodeStatus::InUse { usage } => {
                vec![
                    RawKey::Fixed8([1]),
                    RawKey::Owned(usage.to_be_bytes().to_vec()),
                ]
            },
        }
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        match &bytes[..3] {
            [0, 1, 0] => {
                let since = Duration::from_nanos(u128::from_be_bytes(bytes[3..].try_into()?));
                Ok(CodeStatus::Orphaned { since })
            },
            [0, 1, 1] => {
                let usage = u32::from_be_bytes(bytes[3..].try_into()?);
                Ok(CodeStatus::InUse { usage })
            },
            tag => Err(StdError::deserialize::<Self::Output, _>(
                "key",
                format!("unknown tag: {tag:?}"),
            )),
        }
    }
}

impl<K> PrimaryKey for &K
where
    K: PrimaryKey,
{
    type Output = K::Output;
    type Prefix = K::Prefix;
    type Suffix = K::Suffix;

    const KEY_ELEMS: u8 = 1;

    fn raw_keys(&self) -> Vec<RawKey> {
        (*self).raw_keys()
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        K::from_slice(bytes)
    }
}

impl<A, B> PrimaryKey for (A, B)
where
    A: PrimaryKey + Prefixer,
    B: PrimaryKey,
{
    type Output = (A::Output, B::Output);
    type Prefix = A;
    type Suffix = B;

    const KEY_ELEMS: u8 = A::KEY_ELEMS + B::KEY_ELEMS;

    fn raw_keys(&self) -> Vec<RawKey> {
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

impl<A, B, C> PrimaryKey for (A, B, C)
where
    A: PrimaryKey + Prefixer,
    B: PrimaryKey,
    C: PrimaryKey,
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

    const KEY_ELEMS: u8 = A::KEY_ELEMS + B::KEY_ELEMS + C::KEY_ELEMS;

    fn raw_keys(&self) -> Vec<RawKey> {
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

impl<A, B, C, D> PrimaryKey for (A, B, C, D)
where
    A: PrimaryKey + Prefixer,
    B: PrimaryKey,
    C: PrimaryKey,
    D: PrimaryKey,
{
    type Output = (A::Output, B::Output, C::Output, D::Output);
    type Prefix = A;
    type Suffix = (B, C, D);

    const KEY_ELEMS: u8 = A::KEY_ELEMS + B::KEY_ELEMS + C::KEY_ELEMS + D::KEY_ELEMS;

    fn raw_keys(&self) -> Vec<RawKey> {
        let mut keys = self.0.raw_keys();
        keys.extend(self.1.raw_keys());
        keys.extend(self.2.raw_keys());
        keys.extend(self.3.raw_keys());
        keys
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        let (a_raw, bcd_raw) = split_first_key(A::KEY_ELEMS, bytes);
        let (b_raw, cd_raw) = split_first_key(B::KEY_ELEMS, bcd_raw);
        let (c_raw, d_raw) = split_first_key(C::KEY_ELEMS, cd_raw);

        let a = A::from_slice(&a_raw)?;
        let b = B::from_slice(&b_raw)?;
        let c = C::from_slice(&c_raw)?;
        let d = D::from_slice(d_raw)?;

        Ok((a, b, c, d))
    }
}

// For `Option<T>`s, we treat them basically like a tuple `(u8, T)`.
// The boolean is `0` for `None`, or `1` for `Some(_)`.
impl<T> PrimaryKey for Option<T>
where
    T: PrimaryKey,
{
    type Output = Option<T::Output>;
    type Prefix = bool;
    type Suffix = T;

    const KEY_ELEMS: u8 = 1 + T::KEY_ELEMS;

    fn raw_keys(&self) -> Vec<RawKey> {
        match self {
            Some(k) => {
                let mut keys = true.raw_keys();
                keys.extend(k.raw_keys());
                keys
            },
            None => false.raw_keys(),
        }
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        let (tag, rest) = split_first_key(1, bytes);
        match bool::from_slice(&tag)? {
            true => {
                let inner = T::from_slice(rest)?;
                Ok(Some(inner))
            },
            false => Ok(None),
        }
    }
}

impl<U, const S: u32> PrimaryKey for Dec<U, S>
where
    U: PrimaryKey<Output = U>,
{
    type Output = Self;
    type Prefix = ();
    type Suffix = ();

    const KEY_ELEMS: u8 = 1;

    fn raw_keys(&self) -> Vec<RawKey> {
        self.inner().raw_keys()
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        let numerator = U::from_slice(bytes)?;
        Ok(Self::raw(Int::new(numerator)))
    }
}

impl<U> PrimaryKey for Int<U>
where
    U: PrimaryKey<Output = U>,
{
    type Output = Self;
    type Prefix = ();
    type Suffix = ();

    const KEY_ELEMS: u8 = 1;

    fn raw_keys(&self) -> Vec<RawKey> {
        self.inner().raw_keys()
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        U::from_slice(bytes).map(Self::new)
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
pub(crate) fn split_first_key(key_elems: u8, value: &[u8]) -> (Vec<u8>, &[u8]) {
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

macro_rules! impl_unsigned_integer_key {
    ($t:ty => $variant:ident) => {
        impl PrimaryKey for $t {
            type Output = $t;
            type Prefix = ();
            type Suffix = ();

            const KEY_ELEMS: u8 = 1;

            fn raw_keys(&self) -> Vec<RawKey> {
                vec![RawKey::$variant(self.to_be_bytes())]
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
        }
    };
    ($($t:ty => $variant:ident),+ $(,)?) => {
        $(
            impl_unsigned_integer_key!($t => $variant);
        )*
    };
}

impl_unsigned_integer_key! {
    u8   => Fixed8,
    u16  => Fixed16,
    u32  => Fixed32,
    u64  => Fixed64,
    u128 => Fixed128,
    U256 => Fixed256,
    U512 => Fixed512,
}

macro_rules! impl_signed_integer_key {
    ($s:ty => $u:ty => $variant:ident) => {
        impl PrimaryKey for $s {
            type Output = $s;
            type Prefix = ();
            type Suffix = ();

            const KEY_ELEMS: u8 = 1;

            fn raw_keys(&self) -> Vec<RawKey> {
                let bytes = ((*self as $u) ^ (<$s>::MIN as $u)).to_be_bytes();
                vec![RawKey::$variant(bytes)]
            }

            fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
                let Ok(bytes) = <[u8; mem::size_of::<Self>()]>::try_from(bytes) else {
                    return Err(StdError::deserialize::<Self::Output, _>(
                        "key",
                        format!(
                            "wrong number of bytes: expecting {}, got {}",
                            mem::size_of::<Self>(),
                            bytes.len(),
                        )
                    ));
                };

                Ok((Self::from_be_bytes(bytes) as $u ^ <$s>::MIN as $u) as _)
            }
        }
    };
    ($($s:ty => $u:ty => $variant:ident),+ $(,)?) => {
        $(
            impl_signed_integer_key!($s => $u => $variant);
        )*
    };
}

impl_signed_integer_key! {
    i8   => u8   => Fixed8,
    i16  => u16  => Fixed16,
    i32  => u32  => Fixed32,
    i64  => u64  => Fixed64,
    i128 => u128 => Fixed128,
}

macro_rules! impl_bnum_signed_integer_key {
    ($s:ty => $u:ty => $variant:ident) => {
        impl PrimaryKey for $s {
            type Output = $s;
            type Prefix = ();
            type Suffix = ();

            const KEY_ELEMS: u8 = 1;

            fn raw_keys(&self) -> Vec<RawKey> {
                let bytes = (<$u>::cast_from(self.clone()) ^ <$u>::cast_from(Self::MIN)).to_be_bytes();
                vec![RawKey::$variant(bytes)]
            }

            fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
                let Ok(bytes) = <[u8; mem::size_of::<Self>()]>::try_from(bytes) else {
                    return Err(StdError::deserialize::<Self::Output, _>(
                        "key",
                        format!(
                            "wrong number of bytes: expecting {}, got {}",
                            mem::size_of::<Self>(),
                            bytes.len(),
                        )
                    ));
                };

                Ok(Self::cast_from(
                    <$u>::cast_from(Self::from_be_bytes(bytes)) ^ <$u>::cast_from(Self::MIN)
                ))
            }
        }
    };
    ($($s:ty => $u:ty => $variant:ident),+ $(,)?) => {
        $(
            impl_bnum_signed_integer_key!($s => $u => $variant);
        )*
    };
}

impl_bnum_signed_integer_key! {
    I256 => U256 => Fixed256,
    I512 => U512 => Fixed512,
}

impl<T, B> PrimaryKey for Bounded<T, B>
where
    T: PrimaryKey<Output = T> + PartialOrd + ToString,
    B: Bounds<T>,
{
    type Output = Bounded<T, B>;
    type Prefix = T::Prefix;
    type Suffix = T::Suffix;

    const KEY_ELEMS: u8 = T::KEY_ELEMS;

    fn raw_keys(&self) -> Vec<RawKey> {
        self.inner().raw_keys()
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        T::from_slice(bytes).and_then(Bounded::new)
    }
}

impl<T, const MIN: usize, const MAX: usize> PrimaryKey for LengthBounded<T, MIN, MAX>
where
    T: PrimaryKey<Output = T> + Lengthy,
{
    type Output = LengthBounded<T, MIN, MAX>;
    type Prefix = T::Prefix;
    type Suffix = T::Suffix;

    const KEY_ELEMS: u8 = T::KEY_ELEMS;

    fn raw_keys(&self) -> Vec<RawKey> {
        self.inner().raw_keys()
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        T::from_slice(bytes).and_then(LengthBounded::new)
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        crate::{PrimaryKey, Set},
        bnum::types::I256,
        grug_math::{
            Bytable, Dec128, Dec256, Int64, Int128, Int256, NumberConst, Udec128, Udec256, Uint64,
            Uint128, Uint256, Uint512,
        },
        grug_types::{Addr, Duration, Hash, MockStorage, Order, StdResult},
        std::{fmt::Debug, str::FromStr},
        test_case::test_case,
    };

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

    /// `len(u32) = 4 | 10_u32.to_be_bytes() | 265_u32.to_be_bytes()`
    const DOUBLE_TUPLE_BYTES: &[u8] = &[0, 4, 0, 0, 0, 10, 0, 0, 1, 9];

    /// `len(b"Hello") = 5 | b"Hello" | len(u32) = 4 | 10_u32 | b"World"`
    const DOUBLE_TRIPLE_BYTES: &[u8] = &[
        0, 5, 72, 101, 108, 108, 111, 0, 4, 0, 0, 0, 10, 87, 111, 114, 108, 100,
    ];

    const DEC128_SHIFT: u128 = 10_u128.pow(18);

    #[test_case(
        b"slice".as_slice(),
        b"slice";
        "slice"
    )]
    #[test_case(
        b"Vec".to_vec(),
        b"Vec";
        "vec_u8"
    )]
    #[test_case(
        "str",
        b"str";
        "str"
    )]
    #[test_case(
        "String".to_string(),
        b"String";
        "string"
    )]
    #[test_case(
        Addr::from_inner(*b"ThisIsAValidAddress-"),
        b"ThisIsAValidAddress-";
        "addr"
    )]
    #[test_case(
        &Addr::from_inner(*b"ThisIsAValidAddress-"),
        b"ThisIsAValidAddress-";
        "borrow_addr"
    )]
    #[test_case(
        Hash::<32>::from_inner([1;32]),
        &[1; 32];
        "hash"
    )]
    #[test_case(
        &Hash::<20>::from_inner([2;20]),
        &[2; 20];
        "borrow_hash"
    )]
    #[test_case(
        Duration::from_nanos(100),
        &100_u128.to_be_bytes();
        "duration"
    )]
    #[test_case(
        (10_u32, 265_u32 ),
        &DOUBLE_TUPLE_BYTES;
        "double_tuple"
    )]
    #[test_case(
        ("Hello".to_string(), 10_u32, "World".to_string()),
        &DOUBLE_TRIPLE_BYTES;
        "triple"
    )]
    /// ---- Rust native numbers ----
    #[test_case(
        10_u64,
        &10_u64.to_be_bytes();
        "u64_10"
    )]
    #[test_case(
        -1_i8,
        &[127];
        "i8_neg_1"
    )]
    #[test_case(
        1_i8,
        &[129];
        "i8_1"
    )]
    /// ---- Unsigned integers ----
    #[test_case(
        Uint64::new(10),
        &10_u64.to_be_bytes();
        "uint64_10"
    )]
    #[test_case(
        Uint128::new(10),
        &Uint128::new(10).to_be_bytes();
        "uint128_10"
    )]
    #[test_case(
        Uint256::MIN,
        &[0; 32];
        "uint256_MIN"
    )]
    #[test_case(
        Uint512::MAX,
        &Uint512::MAX.to_be_bytes();
        "uint256_MAX"
    )]
    /// ---- Unsigned Decimals ----
    #[test_case(
        Udec128::from_str("10").unwrap(),
        &(10 * DEC128_SHIFT).to_be_bytes();
        "udec128_10"
    )]
    #[test_case(
        Udec128::from_str("5.5").unwrap(),
        &(5 * DEC128_SHIFT + DEC128_SHIFT / 2).to_be_bytes();
        "udec128_5.5"
    )]
    #[test_case(
        Udec128::MIN,
        &Uint128::MIN.to_be_bytes();
        "udec128_0"
    )]
    #[test_case(
        Udec256::MAX,
        &Uint256::MAX.to_be_bytes();
        "udec256_MAX"
    )]
    #[test_case(
        Int64::new(-10),
        &(-10_i64).joined_key();
        "int64_10"
    )]
    #[test_case(
        Int128::new(-10),
        &(-10_i128).joined_key();
        "int128_neg_10"
    )]
    #[test_case(
        Int256::MIN,
        &(Int256::MIN).joined_key();
        "int256_MIN"
    )]
    #[test_case(
        Int256::MAX,
        &(Int256::MAX).joined_key();
        "int256_MAX"
    )]
    /// ---- Signed Decimals ----
    #[test_case(
        Dec128::MAX,
        &i128::MAX.joined_key();
        "dec128_MAX"
    )]
    #[test_case(
        Dec128::MIN,
        &i128::MIN.joined_key();
        "dec128_MIN"
    )]
    #[test_case(
        Dec256::from_str("-10.5").unwrap(),
        &I256::from(-(105_i128 * 10_i128.pow(17))).joined_key();
        "dec128_neg_10_5"
    )]
    #[test_case(
        Dec256::from_str("20.75").unwrap(),
        &I256::from(2075_i128 * 10_i128.pow(16)).joined_key();
        "dec128_20_75"
    )]
    fn key<T>(compare: T, bytes: &[u8])
    where
        T: PrimaryKey + PartialEq<<T as PrimaryKey>::Output> + Debug,
        <T as PrimaryKey>::Output: Debug,
    {
        let des = T::from_slice(bytes).unwrap();
        assert_eq!(compare, des);

        let ser = compare.joined_key();
        assert_eq!(bytes, ser);
    }

    /// Ensure that when serialized to raw keys, signed integers and decimals
    /// retain their order by value.
    #[test_case(
        [
            Uint128::ZERO,
            Uint128::new(12345),
            Uint128::new(69420),
            Uint128::MAX,
        ];
        "uint128"
    )]
    #[test_case(
        [
            Int128::new(i128::MIN),
            Int128::new(-69420),
            Int128::new(-12345),
            Int128::new(0),
            Int128::new(12345),
            Int128::new(69420),
            Int128::new(i128::MAX),
        ];
        "int128"
    )]
    #[test_case(
        [
            Udec128::ZERO,
            Udec128::checked_from_ratio(Uint128::new(1), Uint128::new(2)).unwrap(),
            Udec128::checked_from_ratio(Uint128::new(1), Uint128::new(1)).unwrap(),
            Udec128::checked_from_ratio(Uint128::new(69420), Uint128::new(12345)).unwrap(),
            Udec128::MAX,
        ];
        "udec128"
    )]
    #[test_case(
        [
            Dec128::MIN,
            Dec128::checked_from_ratio(Int128::new(-69420), Int128::new(12345)).unwrap(),
            Dec128::checked_from_ratio(Int128::new(-1), Int128::new(1)).unwrap(),
            Dec128::checked_from_ratio(Int128::new(-1), Int128::new(2)).unwrap(),
            Dec128::new(0_i128),
            Dec128::checked_from_ratio(Int128::new(1), Int128::new(2)).unwrap(),
            Dec128::checked_from_ratio(Int128::new(1), Int128::new(1)).unwrap(),
            Dec128::checked_from_ratio(Int128::new(69420), Int128::new(12345)).unwrap(),
            Dec128::MAX,
        ];
        "dec128"
    )]
    fn number_key_ordering<T, const N: usize>(numbers: [T; N])
    where
        T: PrimaryKey + PartialEq<<T as PrimaryKey>::Output> + Debug + Copy,
        <T as PrimaryKey>::Output: Debug,
    {
        let set = Set::<T>::new("numbers");

        let mut storage = MockStorage::new();

        // Now save these keys in the KV store.
        for number in numbers {
            set.insert(&mut storage, number).unwrap();
        }

        // Fetch the numbers in ascending order. Should match the original
        // array.
        {
            let recovered = set
                .range(&storage, None, None, Order::Ascending)
                .collect::<StdResult<Vec<_>>>()
                .unwrap();

            assert_eq!(numbers, recovered.as_slice());
        }

        // Fetch the numbers in descending order. Should be the original array
        // in reverse.
        {
            let mut recovered = set
                .range(&storage, None, None, Order::Descending)
                .collect::<StdResult<Vec<_>>>()
                .unwrap();

            recovered.reverse();

            assert_eq!(numbers, recovered.as_slice());
        }
    }
}
