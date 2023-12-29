use {
    anyhow::{bail, Context},
    data_encoding::BASE64,
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

impl<'a> RawKey<'a> {
    pub fn len(&self) -> usize {
        match self {
            RawKey::Owned(vec) => vec.len(),
            RawKey::Ref(slice) => slice.len(),
            RawKey::Val8(_)    => 1,
            RawKey::Val16(_)   => 2,
            RawKey::Val32(_)   => 4,
            RawKey::Val64(_)   => 8,
            RawKey::Val128(_)  => 16,
        }
    }
}

impl<'a> AsRef<[u8]> for RawKey<'a> {
    fn as_ref(&self) -> &[u8] {
        match self {
            RawKey::Owned(vec)    => vec,
            RawKey::Ref(slice)    => slice,
            RawKey::Val8(slice)   => slice,
            RawKey::Val16(slice)  => slice,
            RawKey::Val32(slice)  => slice,
            RawKey::Val64(slice)  => slice,
            RawKey::Val128(slice) => slice,
        }
    }
}

// a map key needs to be serialized to or deserialized from raw bytes. however,
// we don't want to rely on serde traits here because it's slow, not compact,
// and faillable.
pub trait MapKey: Sized {
    // for compound keys, the first element; e.g. for (A, B), A is the prefix.
    // for single keys, use ()
    type Prefix: MapKey;

    // for compound keys, the elements minus the first one; e.g. for (A, B), B is the suffix.
    // for single keys, use ()
    type Suffix: MapKey;

    fn serialize(&self) -> Vec<RawKey>;

    fn deserialize(bytes: &[u8]) -> anyhow::Result<Self>;
}

impl MapKey for () {
    type Prefix = ();
    type Suffix = ();

    fn serialize(&self) -> Vec<RawKey> {
        vec![RawKey::Ref(&[])]
    }

    fn deserialize(bytes: &[u8]) -> anyhow::Result<Self> {
        if !bytes.is_empty() {
            bail!(
                "Failed to deserialize into empty map key: expecting empty bytes, got {}",
                BASE64.encode(bytes),
            );
        }

        Ok(())
    }
}

impl MapKey for Vec<u8> {
    type Prefix = ();
    type Suffix = ();

    fn serialize(&self) -> Vec<RawKey> {
        vec![RawKey::Ref(self.as_slice())]
    }

    fn deserialize(bytes: &[u8]) -> anyhow::Result<Self> {
        Ok(bytes.to_vec())
    }
}

impl MapKey for String {
    type Prefix = ();
    type Suffix = ();

    fn serialize(&self) -> Vec<RawKey> {
        vec![RawKey::Ref(self.as_bytes())]
    }

    fn deserialize(bytes: &[u8]) -> anyhow::Result<Self> {
        String::from_utf8(bytes.to_vec())
            .context("Failed to deserialize into String map key")
    }
}

macro_rules! map_integer_map_key {
    (for $($t:ty, $v:tt),+ $(,)?) => {
        $(impl MapKey for $t {
            type Prefix = ();
            type Suffix = ();

            fn serialize(&self) -> Vec<RawKey> {
                vec![RawKey::$v(self.to_be_bytes())]
            }

            fn deserialize(bytes: &[u8]) -> anyhow::Result<Self> {
                <[u8; mem::size_of::<Self>()]>::try_from(bytes)
                    .map(Self::from_be_bytes)
                    .context("Failed to deserialize into $t map key")
            }
        })*
    }
}

map_integer_map_key!(for
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

    fn serialize(&self) -> Vec<RawKey> {
        let mut keys = vec![];
        keys.extend(self.0.serialize());
        keys.extend(self.1.serialize());
        keys
    }

    fn deserialize(bytes: &[u8]) -> anyhow::Result<Self> {
        todo!()
    }
}
