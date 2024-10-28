use {
    crate::PrimaryKey,
    grug_math::{
        Bytable, Dec, Fraction, Int, Int128, Int256, Int512, Int64, Uint128, Uint256, Uint512,
        Uint64,
    },
    grug_types::{nested_namespaces_with_key, Addr, CodeStatus, Denom, Duration, Hash, Part},
    std::{borrow::Cow, str, vec},
};

/// Describes a prefix in a composite key.
///
/// This trait is different from [`PrimaryKey`](crate::PrimaryKey) in two way:
///
/// 1. `PrimaryKey::joined_key` prefixes each raw key except for the last one
///    with their lengths. However, `Prefixer::joined_prefix` prefixes all raw
///    prefixes with lengths, including the last one.
/// 2. `Prefixer` does not have a `from_slice` method because not needed.
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

impl Prefixer for Part {
    fn raw_prefixes(&self) -> Vec<Cow<[u8]>> {
        vec![Cow::Borrowed(self.as_bytes())]
    }
}

impl Prefixer for Denom {
    fn raw_prefixes(&self) -> Vec<Cow<[u8]>> {
        vec![Cow::Owned(self.to_string().into_bytes())]
    }
}

impl Prefixer for Duration {
    fn raw_prefixes(&self) -> Vec<Cow<[u8]>> {
        vec![Cow::Owned(self.into_nanos().to_be_bytes().to_vec())]
    }
}

impl Prefixer for CodeStatus {
    fn raw_prefixes(&self) -> Vec<Cow<[u8]>> {
        self.raw_keys()
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

impl<T> Prefixer for Dec<T>
where
    Int<T>: PrimaryKey<Output = Int<T>>,
    Dec<T>: Fraction<T>,
{
    fn raw_prefixes(&self) -> Vec<Cow<[u8]>> {
        self.raw_keys()
    }
}

macro_rules! impl_integer_prefixer {
    ($t:ty) => {
        impl Prefixer for $t {
            fn raw_prefixes(&self) -> Vec<Cow<[u8]>> {
                vec![Cow::Owned(self.to_be_bytes().to_vec())]
            }
        }
    };
    ($($t:ty),+ $(,)?) => {
        $(
            impl_integer_prefixer!($t);
        )*
    };
}

impl_integer_prefixer! {
    u8, u16, u32, u64, u128,
    i8, i16, i32, i64, i128,
    Uint64, Uint128, Uint256, Uint512,
    Int64, Int128, Int256, Int512,
}
