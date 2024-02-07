use {
    crate::BitArray,
    cw_std::{Hash, MapKey, RawKey, StdError, StdResult},
    std::mem,
};

// we need to serialize NodeKey into binary so that it can be used as keys in
// the backing KV store.
// since we use a 32-byte hash, the NodeKey serializes to 10-42 bytes:
// - the first 8 bytes are the version in big endian
// - the next 2 bytes are the num_bits in big endian
// - the rest 0-32 bits are the bits
//
// ********|**|********************************
// ^       ^  ^                               ^
// 0       b1 b2                              b3
const LEN_1: usize = mem::size_of::<u64>();         // 8
const LEN_2: usize = LEN_1 + mem::size_of::<u16>(); // 10
const LEN_3: usize = LEN_2 + Hash::LENGTH;          // 42

pub struct NodeKey {
    pub version: u64,
    pub bits:    BitArray,
}

impl NodeKey {
    pub fn root(version: u64) -> Self {
        Self {
            version,
            bits: BitArray::empty(),
        }
    }

    pub fn child_at_version(&self, left: bool, version: u64) -> Self {
        let mut bits = self.bits.clone();
        bits.push(if left { 0 } else { 1 });
        Self { version, bits }
    }
}

impl MapKey for &NodeKey {
    type Prefix = ();
    type Suffix = ();
    type Output = NodeKey;

    /// Assuming a 32-byte hash is used, the NodeKey serializes to 10-42 bytes:
    /// - the first 8 bytes are the version in big endian
    /// - the next 2 bytes are the num_bits in big endian
    /// - the rest 0-32 bits are the bits
    fn raw_keys(&self) -> Vec<RawKey> {
        // how many bytes are necesary to represent the bits
        let num_bytes = self.bits.num_bits.div_ceil(8);
        let mut bytes = Vec::with_capacity(num_bytes + 10);
        bytes.extend(self.version.to_be_bytes());
        bytes.extend(self.bits.num_bits.to_be_bytes());
        bytes.extend(&self.bits.bytes[..num_bytes]);
        vec![RawKey::Owned(bytes)]
    }

    fn deserialize(slice: &[u8]) -> StdResult<Self::Output> {
        let range = LEN_1..=LEN_3;
        if !range.contains(&slice.len()) {
            return Err(StdError::deserialize::<Self>(
                format!("slice length must be in the range {range:?}, found {}", slice.len())
            ));
        }

        let version = u64::from_be_bytes(slice[..LEN_1].try_into()?);
        let num_bits = u16::from_be_bytes(slice[LEN_1..LEN_2].try_into()?) as usize;
        let bytes = slice[LEN_2..].try_into()?;

        Ok(NodeKey {
            version,
            bits: BitArray { num_bits, bytes },
        })
    }
}
