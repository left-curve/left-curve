use {
    grug_types::{nested_namespaces_with_key, split_one_key, Addr, Hash, StdError, StdResult},
    std::{borrow::Cow, mem},
};

macro_rules! empty_prefix_suffix {
    () => {
        fn index_prefix(&self) -> Self::IndexPrefix {
            ()
        }

        fn index_suffix(&self) -> Self::IndexSuffix {
            self
        }
    };
    ($fn:ident) => {
        fn index_prefix(&self) -> Self::IndexPrefix {
            ()
        }

        fn index_suffix(&self) -> Self::IndexSuffix {
            self.$fn()
        }
    };
}

/// A raw storage key is a byte slice, either owned or borrowed.
pub type RawKey<'a> = Cow<'a, [u8]>;

/// Describes a key used in mapping data structure.
///
/// The key needs to be serialized to or deserialized from raw bytes. However,
/// we don't want to use `serde` here because it's slow, not compact, and
/// faillable.
///
/// Additionally, compound keys can be split into `Prefix` and `Suffix`, which
/// are useful in iterations.
pub trait Key {
    /// For compound keys, the first element; e.g. for `(A, B)`, `A` is the
    /// prefix. For single keys, use `()`.
    type Prefix: Key;

    /// For compound keys, the elements minus the first one; e.g. for `(A, B)`,
    /// `B` is the suffix. For single keys, use ().
    type Suffix: Key;

    /// Prefix used on multi index.
    /// For single keys, use `()`.
    /// For compound keys, use the first half elements; e.g. for `(A, B)`, `A` is the
    type IndexPrefix: Key;

    /// Suffix used on multi index.
    /// For single keys, use Self.
    /// For compound keys, use the second half elements; e.g. for `(A, B)`, `B` is the
    type IndexSuffix: Key;

    /// The type the deserialize into, which may be different from the key
    /// itself.
    ///
    /// E.g. use `&str` as the key but deserializes into `String`.
    ///
    /// Note: The output must be an owned type. in comparison, the key itself is
    /// almost always a reference type or a copy-able type.
    type Output: 'static;

    fn raw_keys(&self) -> Vec<RawKey>;

    fn serialize(&self) -> Vec<u8> {
        let mut raw_keys = self.raw_keys();
        let last_raw_key = raw_keys.pop();
        nested_namespaces_with_key(None, &raw_keys, last_raw_key.as_ref())
    }

    fn deserialize(bytes: &[u8]) -> StdResult<Self::Output>;

    fn joined_extra_key(&self, key: &[u8]) -> Vec<u8> {
        nested_namespaces_with_key(None, &self.raw_keys(), Some(&key))
    }

    fn index_prefix(&self) -> Self::IndexPrefix;

    fn index_suffix(&self) -> Self::IndexSuffix;

    /// Deserialization fn used on MultiIndex.
    /// When IndexMap serialize the key, it serialize the Index::Prefix and Index::Suffix.
    /// On Non tuple keys, the Index::Prefix has to be equal () and Index::Suffix is the key.
    /// We need to trim the 2 bytes that is the len of the Index::Prefix (0).
    ///
    /// On tuples, this function has to be overriden and just return `Self::deserialize(bytes)`
    fn deserialize_from_index(bytes: &[u8]) -> StdResult<Self::Output> {
        Self::deserialize(&bytes[2..])
    }

    /// Adjustment fn used on MultiIndex for load the value from primary key.
    /// When IndexMap serialize the key, it serialize the Index::Prefix and Index::Suffix.
    /// On Non tuple keys, the Index::Prefix has to be equal () and Index::Suffix is the key.
    /// We need to trim the 2 bytes that is the len of the Index::Prefix (0).
    ///
    /// On tuples, this function has to be overriden and just return `bytes`
    fn adjust_from_index(bytes: &[u8]) -> &[u8] {
        &bytes[2..]
    }
}

impl Key for () {
    type IndexPrefix = ();
    type IndexSuffix = ();
    type Output = ();
    type Prefix = ();
    type Suffix = ();

    empty_prefix_suffix!(clone);

    fn raw_keys(&self) -> Vec<RawKey> {
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
    type IndexPrefix = ();
    type IndexSuffix = Vec<u8>;
    type Output = Vec<u8>;
    type Prefix = ();
    type Suffix = ();

    empty_prefix_suffix!(to_vec);

    fn raw_keys(&self) -> Vec<RawKey> {
        vec![RawKey::Borrowed(self)]
    }

    fn deserialize(bytes: &[u8]) -> StdResult<Self::Output> {
        Ok(bytes.to_vec())
    }
}

impl Key for Vec<u8> {
    type IndexPrefix = ();
    type IndexSuffix = Vec<u8>;
    type Output = Vec<u8>;
    type Prefix = ();
    type Suffix = ();

    empty_prefix_suffix!(clone);

    fn raw_keys(&self) -> Vec<RawKey> {
        vec![RawKey::Borrowed(self)]
    }

    fn deserialize(bytes: &[u8]) -> StdResult<Self::Output> {
        Ok(bytes.to_vec())
    }
}

impl Key for &str {
    type IndexPrefix = ();
    type IndexSuffix = String;
    type Output = String;
    type Prefix = ();
    type Suffix = ();

    empty_prefix_suffix!(to_string);

    fn raw_keys(&self) -> Vec<RawKey> {
        vec![RawKey::Borrowed(self.as_bytes())]
    }

    fn deserialize(bytes: &[u8]) -> StdResult<Self::Output> {
        String::from_utf8(bytes.to_vec()).map_err(StdError::deserialize::<Self::Output>)
    }
}

impl<'a> Key for &'a Addr {
    type IndexPrefix = ();
    type IndexSuffix = &'a Addr;
    type Output = Addr;
    type Prefix = ();
    type Suffix = ();

    empty_prefix_suffix!();

    fn raw_keys(&self) -> Vec<RawKey> {
        vec![RawKey::Borrowed(self.as_ref())]
    }

    fn deserialize(bytes: &[u8]) -> StdResult<Self::Output> {
        bytes.try_into()
    }
}

impl<'a> Key for &'a Hash {
    type IndexPrefix = ();
    type IndexSuffix = &'a Hash;
    type Output = Hash;
    type Prefix = ();
    type Suffix = ();

    empty_prefix_suffix!();

    fn raw_keys(&self) -> Vec<RawKey> {
        vec![RawKey::Borrowed(self.as_ref())]
    }

    fn deserialize(bytes: &[u8]) -> StdResult<Self::Output> {
        bytes.try_into()
    }
}

impl Key for String {
    type IndexPrefix = ();
    type IndexSuffix = String;
    type Output = String;
    type Prefix = ();
    type Suffix = ();

    empty_prefix_suffix!(clone);

    fn raw_keys(&self) -> Vec<RawKey> {
        vec![RawKey::Borrowed(self.as_bytes())]
    }

    fn deserialize(bytes: &[u8]) -> StdResult<Self::Output> {
        String::from_utf8(bytes.to_vec()).map_err(StdError::deserialize::<Self::Output>)
    }
}

macro_rules! impl_integer_map_key {
    ($($t:ty),+ $(,)?) => {
        $(impl Key for $t {
            type IndexPrefix = ();
            type IndexSuffix = $t;
            type Prefix = ();
            type Suffix = ();
            type Output = $t;

            fn raw_keys(&self) -> Vec<RawKey> {
                vec![RawKey::Owned(self.to_be_bytes().to_vec())]
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

            empty_prefix_suffix!(clone);
        })*
    }
}

impl_integer_map_key!(i8, u8, i16, u16, i32, u32, i64, u64, i128, u128);

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
    A: Key + Clone,
    B: Key + Clone,
{
    type IndexPrefix = A;
    type IndexSuffix = B;
    type Output = (A::Output, B::Output);
    type Prefix = A;
    type Suffix = B;

    fn raw_keys(&self) -> Vec<RawKey> {
        let a = self.0.serialize();
        let b = self.1.serialize();
        vec![RawKey::Owned(a), RawKey::Owned(b)]
    }

    fn deserialize(bytes: &[u8]) -> StdResult<Self::Output> {
        let (a_bytes, b_bytes) = split_one_key(bytes);
        let a = A::deserialize(a_bytes)?;
        let b = B::deserialize(b_bytes)?;
        Ok((a, b))
    }

    fn index_prefix(&self) -> Self::IndexPrefix {
        self.0.clone()
    }

    fn index_suffix(&self) -> Self::IndexSuffix {
        self.1.clone()
    }

    fn deserialize_from_index(bytes: &[u8]) -> StdResult<Self::Output> {
        Self::deserialize(bytes)
    }

    fn adjust_from_index(bytes: &[u8]) -> &[u8] {
        bytes
    }
}

impl<A, B, C> Key for (A, B, C)
where
    A: Key + Clone,
    B: Key + Clone,
    C: Key + Clone,
{
    type IndexPrefix = A;
    type IndexSuffix = (B, C);
    type Output = (A::Output, B::Output, C::Output);
    type Prefix = A;
    type Suffix = (B, C);

    fn raw_keys(&self) -> Vec<RawKey> {
        let a = self.0.serialize();
        let b = self.1.serialize();
        let c = self.2.serialize();
        vec![RawKey::Owned(a), RawKey::Owned(b), RawKey::Owned(c)]
    }

    fn deserialize(bytes: &[u8]) -> StdResult<Self::Output> {
        let (a_bytes, bc_bytes) = split_one_key(bytes);
        let (b_bytes, c_bytes) = split_one_key(bc_bytes);
        let a = A::deserialize(a_bytes)?;
        let b = B::deserialize(b_bytes)?;
        let c = C::deserialize(c_bytes)?;
        Ok((a, b, c))
    }

    fn index_prefix(&self) -> Self::IndexPrefix {
        self.0.clone()
    }

    fn index_suffix(&self) -> Self::IndexSuffix {
        (self.1.clone(), self.2.clone())
    }

    fn deserialize_from_index(_bytes: &[u8]) -> StdResult<Self::Output> {
        todo!("tuple 3 can be implemented as MultiIndex key??")
    }

    fn adjust_from_index(_bytes: &[u8]) -> &[u8] {
        todo!("tuple 3 can be implemented as MultiIndex key??")
    }
}

impl<A, B, C, D> Key for (A, B, C, D)
where
    A: Key + Clone,
    B: Key + Clone,
    C: Key + Clone,
    D: Key + Clone,
{
    type IndexPrefix = (A, B);
    type IndexSuffix = (C, D);
    type Output = (A::Output, B::Output, C::Output, D::Output);
    type Prefix = (A, B);
    type Suffix = (C, D);

    fn raw_keys(&self) -> Vec<RawKey> {
        let a = self.0.serialize();
        let b = self.1.serialize();
        let c = self.2.serialize();
        let d = self.3.serialize();

        vec![
            RawKey::Owned(a),
            RawKey::Owned(b),
            RawKey::Owned(c),
            RawKey::Owned(d),
        ]
    }

    fn deserialize(bytes: &[u8]) -> StdResult<Self::Output> {
        let (a_bytes, bc_bytes) = split_one_key(bytes);
        let (b_bytes, cd_bytes) = split_one_key(bc_bytes);
        let (c_bytes, d_bytes) = split_one_key(cd_bytes);

        let a = A::deserialize(a_bytes)?;
        let b = B::deserialize(b_bytes)?;
        let c = C::deserialize(c_bytes)?;
        let d = D::deserialize(d_bytes)?;

        Ok((a, b, c, d))
    }

    fn index_prefix(&self) -> Self::IndexPrefix {
        (self.0.clone(), self.1.clone())
    }

    fn index_suffix(&self) -> Self::IndexSuffix {
        (self.2.clone(), self.3.clone())
    }

    fn deserialize_from_index(_bytes: &[u8]) -> StdResult<Self::Output> {
        todo!("tuple 4 can be implemented as MultiIndex key??")
    }

    fn adjust_from_index(_bytes: &[u8]) -> &[u8] {
        todo!("tuple 4 can be implemented as MultiIndex key??")
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
#[rustfmt::skip]
mod tests {
    use super::*;

    #[test]
    fn triple_tuple_key() {
        type TripleTuple<'a> = (&'a str, &'a str, &'a str);

        let (a, b, c) = ("larry", "jake", "pumpkin");
        let serialized = (a, b, c).serialize();
        let deserialized = TripleTuple::deserialize(&serialized).unwrap();

        assert_eq!(
            deserialized,
            (a.to_string(), b.to_string(), c.to_string()),
        );
    }

    #[test]
    fn nested_tuple_key() {
        type NestedTuple<'a> = ((&'a str, &'a str), (&'a str, &'a str));

        let ((a, b), (c, d)) = (("larry", "engineer"), ("jake", "shepherd"));
        let serialized = ((a, b), (c, d)).serialize();
        let deserialized = NestedTuple::deserialize(&serialized).unwrap();

        assert_eq!(
            deserialized,
            ((a.to_string(), b.to_string()), (c.to_string(), d.to_string()))
        );
    }
}
