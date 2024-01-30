use {
    crate::{nested_namespaces_with_key, split_one_key, StdError, StdResult},
    std::mem,
};

pub enum RawKey<'a> {
    Owned(Vec<u8>),
    Ref(&'a [u8]),
    Val8([u8; 1]),
    Val16([u8; 2]),
    Val32([u8; 4]),
    Val64([u8; 8]),
    Val128([u8; 16]),
}

impl<'a> AsRef<[u8]> for RawKey<'a> {
    fn as_ref(&self) -> &[u8] {
        match self {
            RawKey::Owned(vec) => vec,
            RawKey::Ref(slice) => slice,
            RawKey::Val8(slice) => slice,
            RawKey::Val16(slice) => slice,
            RawKey::Val32(slice) => slice,
            RawKey::Val64(slice) => slice,
            RawKey::Val128(slice) => slice,
        }
    }
}

// a map key needs to be serialized to or deserialized from raw bytes. however,
// we don't want to rely on serde traits here because it's slow, not compact,
// and faillable.
pub trait MapKey: Sized {
    // for compound keys, the first element; e.g. for (A, B), A is the prefix.
    // for single keys, use ().
    type Prefix: MapKey;

    // for compound keys, the elements minus the first one; e.g. for (A, B), B is the suffix.
    // for single keys, use ().
    type Suffix: MapKey;

    // the type the deserialize into, which may be different from the key itself.
    // e.g. use &str as the key but deserializes into String.
    //
    // NOTE: the output must be an owned type. in comparison, the key itself is
    // almost always a reference type or a copy-able type.
    type Output: 'static;

    fn raw_keys(&self) -> Vec<RawKey>;

    fn serialize(&self) -> Vec<u8> {
        let mut raw_keys = self.raw_keys();
        let last_raw_key = raw_keys.pop();
        nested_namespaces_with_key(None, &raw_keys, last_raw_key.as_ref())
    }

    fn deserialize(bytes: &[u8]) -> StdResult<Self::Output>;
}

impl MapKey for () {
    type Prefix = ();
    type Suffix = ();
    type Output = ();

    fn raw_keys(&self) -> Vec<RawKey> {
        vec![]
    }

    fn deserialize(bytes: &[u8]) -> StdResult<Self::Output> {
        if !bytes.is_empty() {
            return Err(StdError::deserialize::<Self::Output>("expecting empty bytes"));
        }

        Ok(())
    }
}

// TODO: create a Binary type and replace this with &Binary
impl MapKey for &[u8] {
    type Prefix = ();
    type Suffix = ();
    type Output = Vec<u8>;

    fn raw_keys(&self) -> Vec<RawKey> {
        vec![RawKey::Ref(self)]
    }

    fn deserialize(bytes: &[u8]) -> StdResult<Self::Output> {
        Ok(bytes.to_vec())
    }
}

impl MapKey for &str {
    type Prefix = ();
    type Suffix = ();
    type Output = String;

    fn raw_keys(&self) -> Vec<RawKey> {
        vec![RawKey::Ref(self.as_bytes())]
    }

    fn deserialize(bytes: &[u8]) -> StdResult<Self::Output> {
        String::from_utf8(bytes.to_vec()).map_err(StdError::deserialize::<Self::Output>)
    }
}

macro_rules! impl_integer_map_key {
    ($($t:ty, $v:tt),+ $(,)?) => {
        $(impl MapKey for $t {
            type Prefix = ();
            type Suffix = ();
            type Output = $t;

            fn raw_keys(&self) -> Vec<RawKey> {
                vec![RawKey::$v(self.to_be_bytes())]
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
        })*
    }
}

impl_integer_map_key!(
    i8,   Val8,   u8,   Val8,
    i16,  Val16,  u16,  Val16,
    i32,  Val32,  u32,  Val32,
    i64,  Val64,  u64,  Val64,
    i128, Val128, u128, Val128,
);

impl<A, B> MapKey for (A, B)
where
    A: MapKey,
    B: MapKey,
{
    type Prefix = A;
    type Suffix = B;
    type Output = (A::Output, B::Output);

    fn raw_keys(&self) -> Vec<RawKey> {
        let mut keys = vec![];
        keys.extend(self.0.raw_keys());
        keys.extend(self.1.raw_keys());
        keys
    }

    fn deserialize(bytes: &[u8]) -> StdResult<Self::Output> {
        let (a_bytes, b_bytes) = split_one_key(bytes);
        let a = A::deserialize(a_bytes)?;
        let b = B::deserialize(b_bytes)?;
        Ok((a, b))
    }
}
