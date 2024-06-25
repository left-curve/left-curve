use {
    grug_types::{nested_namespaces_with_key, Addr, Hash, StdError, StdResult, VecExt},
    std::{borrow::Cow, mem},
};

/// Describes a key used in mapping data structure.
///
/// The key needs to be serialized to or deserialized from raw bytes. However,
/// we don't want to use `serde` here because it's slow, not compact, and
/// faillable.
///
/// Additionally, compound keys can be split into `Prefix` and `Suffix`, which
/// are useful in iterations.
pub trait Key {
    /// The number of keys in this compound key. For single keys, this is 1.
    const KEYS: u16 = 1;

    /// For compound keys, the first element; e.g. for `(A, B)`, `A` is the
    /// prefix. For single keys, use `()`.
    type Prefix: Key;

    /// For compound keys, the elements minus the first one; e.g. for `(A, B)`,
    /// `B` is the suffix. For single keys, use ().
    type Suffix: Key;

    /// The type the deserialize into, which may be different from the key
    /// itself.
    ///
    /// E.g. use `&str` as the key but deserializes into `String`.
    ///
    /// Note: The output must be an owned type. in comparison, the key itself is
    /// almost always a reference type or a copy-able type.
    type Output: 'static;

    fn raw_keys(&self) -> Vec<Cow<[u8]>>;

    fn serialize(&self) -> Vec<u8> {
        let mut raw_keys = self.raw_keys();
        let last_raw_key = raw_keys.pop();
        nested_namespaces_with_key(None, &raw_keys, last_raw_key.as_ref())
    }

    fn deserialize(bytes: &[u8]) -> StdResult<Self::Output>;

    fn joined_extra_key(&self, key: &[u8]) -> Vec<u8> {
        nested_namespaces_with_key(None, &self.raw_keys(), Some(&key))
    }
}

impl Key for () {
    type Output = ();
    type Prefix = ();
    type Suffix = ();

    const KEYS: u16 = 0;

    fn raw_keys(&self) -> Vec<Cow<[u8]>> {
        vec![]
    }

    fn deserialize(bytes: &[u8]) -> StdResult<Self::Output> {
        if !bytes.is_empty() {
            return Err(StdError::deserialize::<Self::Output>(
                "expecting empty bytes",
            ));
        }

        Ok(())
    }
}

// TODO: create a Binary type and replace this with &Binary
impl Key for &[u8] {
    type Output = Vec<u8>;
    type Prefix = ();
    type Suffix = ();

    fn raw_keys(&self) -> Vec<Cow<[u8]>> {
        vec![Cow::Borrowed(self)]
    }

    fn deserialize(bytes: &[u8]) -> StdResult<Self::Output> {
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

    fn deserialize(bytes: &[u8]) -> StdResult<Self::Output> {
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

    fn deserialize(bytes: &[u8]) -> StdResult<Self::Output> {
        String::from_utf8(bytes.to_vec()).map_err(StdError::deserialize::<Self::Output>)
    }
}

impl<'a> Key for &'a Addr {
    type Output = Addr;
    type Prefix = ();
    type Suffix = ();

    fn raw_keys(&self) -> Vec<Cow<[u8]>> {
        vec![Cow::Borrowed(self.as_ref())]
    }

    fn deserialize(bytes: &[u8]) -> StdResult<Self::Output> {
        bytes.try_into()
    }
}

impl<'a> Key for &'a Hash {
    type Output = Hash;
    type Prefix = ();
    type Suffix = ();

    fn raw_keys(&self) -> Vec<Cow<[u8]>> {
        vec![Cow::Borrowed(self.as_ref())]
    }

    fn deserialize(bytes: &[u8]) -> StdResult<Self::Output> {
        bytes.try_into()
    }
}

impl Key for String {
    type Output = String;
    type Prefix = ();
    type Suffix = ();

    fn raw_keys(&self) -> Vec<Cow<[u8]>> {
        vec![Cow::Borrowed(self.as_bytes())]
    }

    fn deserialize(bytes: &[u8]) -> StdResult<Self::Output> {
        String::from_utf8(bytes.to_vec()).map_err(StdError::deserialize::<Self::Output>)
    }
}

fn split_first_key(key_elems: u16, value: &[u8]) -> StdResult<(Vec<u8>, &[u8])> {
    let mut index = 0;
    let mut first_key = Vec::new();

    // Iterate over the sub keys
    for i in 0..key_elems {
        let len_slice = &value[index..index + 2];
        index += 2;
        let is_last_key = i == key_elems - 1;

        if !is_last_key {
            first_key.extend_from_slice(len_slice);
        }

        let subkey_len = u16::from_be_bytes(len_slice.try_into()?) as usize;
        first_key.extend_from_slice(&value[index..index + subkey_len]);
        index += subkey_len;
    }

    let remainder = &value[index..];
    Ok((first_key, remainder))
}

// Our implementation of serializing tuple keys is different from CosmWasm's,
// because theirs doesn't work for nested tuples:
// <https://github.com/CosmWasm/cw-storage-plus/issues/81>
//
// For example, consider the following key: `((A, B), (C, D))`. With CosmWasm's
// implementation, it will be serialized as:
//
// len(A) | A | len(B) | B | len(C) | C | D
//
// When deserializing, the contract doesn't know where (A, B) ends and where
// (C, D) starts, which results in errors.
//
// With our implementation, this is deserialized as:
//
// len(A+B) | len(A) | A | B | len(C) | C | D
//
// There is no ambiguity, and deserialization works.
//
// See the `nested_tuple_key` test at the bottom of this file for a demo.
impl<A, B> Key for (A, B)
where
    A: Key,
    B: Key,
{
    type Output = (A::Output, B::Output);
    type Prefix = A;
    type Suffix = B;

    const KEYS: u16 = A::KEYS + B::KEYS;

    fn raw_keys(&self) -> Vec<Cow<[u8]>> {
        self.0.raw_keys().merge(self.1.raw_keys())
    }

    fn deserialize(bytes: &[u8]) -> StdResult<Self::Output> {
        let (a, b) = split_first_key(A::KEYS, bytes)?;
        Ok((A::deserialize(&a)?, B::deserialize(b)?))
    }
}

impl<A, B, C> Key for (A, B, C)
where
    A: Key,
    B: Key,
    C: Key,
{
    type Output = (A::Output, B::Output, C::Output);
    type Prefix = A;
    type Suffix = (B, C);

    const KEYS: u16 = A::KEYS + B::KEYS + C::KEYS;

    fn raw_keys(&self) -> Vec<Cow<[u8]>> {
        self.0
            .raw_keys()
            .merge(self.1.raw_keys())
            .merge(self.2.raw_keys())
    }

    fn deserialize(bytes: &[u8]) -> StdResult<Self::Output> {
        let (a, bc) = split_first_key(A::KEYS, bytes)?;
        let (b, c) = split_first_key(B::KEYS, bc)?;

        Ok((A::deserialize(&a)?, B::deserialize(&b)?, C::deserialize(c)?))
    }
}

macro_rules! impl_integer_map_key {
    ($($t:ty),+ $(,)?) => {
        $(impl Key for $t {

            type Prefix = ();
            type Suffix = ();
            type Output = $t;

            fn raw_keys(&self) -> Vec<Cow<[u8]>> {
                vec![Cow::Owned(self.to_be_bytes().to_vec())]
            }

            fn deserialize(bytes: &[u8]) -> StdResult<Self::Output> {
                let Ok(bytes) = <[u8; mem::size_of::<Self>()]>::try_from(bytes) else {
                    return Err(StdError::deserialize::<Self::Output>(format!(
                        "wrong number of bytes: expecting {}, got {}",
                        mem::size_of::<Self>(),
                        bytes.len(),
                    )));
                };

                Ok(Self::from_be_bytes(bytes))
            }

        }

    )*
    }
}

impl_integer_map_key!(i8, u8, i16, u16, i32, u32, i64, u64, i128, u128);

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triple_tuple_key() {
        type TripleTuple<'a> = (&'a str, &'a str, &'a str);

        let (a, b, c) = ("larry", "jake", "pumpkin");
        let serialized = (a, b, c).serialize();
        let deserialized = TripleTuple::deserialize(&serialized).unwrap();

        assert_eq!(deserialized, (a.to_string(), b.to_string(), c.to_string()),);
    }

    #[test]
    fn nested_tuple_key() {
        type NestedTuple<'a> = ((&'a str, &'a str), (&'a str, &'a str));
        let ((a, b), (c, d)) = (("larry", "engineer"), ("jake", "shepherd"));
        let serialized = ((a, b), (c, d)).serialize();
        let deserialized = NestedTuple::deserialize(&serialized).unwrap();

        assert_eq!(
            deserialized,
            (
                (a.to_string(), b.to_string()),
                (c.to_string(), d.to_string())
            )
        );
    }
}
