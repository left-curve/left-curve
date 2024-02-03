use {
    crate::BitArray,
    cw_std::{MapKey, RawKey, StdError, StdResult},
};

pub struct NodeKey<const N: usize = 32> {
    pub version: u64,
    pub bits:    BitArray<N>,
}

impl<const N: usize> NodeKey<N> {
    pub fn root(version: u64) -> Self {
        Self {
            version,
            bits: BitArray::empty(),
        }
    }
}

impl<const N: usize> MapKey for &NodeKey<N> {
    type Prefix = ();
    type Suffix = ();
    type Output = NodeKey<N>;

    /// Assuming a 32-byte hash is used, the NodeKey serializes to 10-42 bytes:
    /// - the first 8 bytes are the version in big endian
    /// - the next 2 bytes are the num_bits in big endian
    /// - the rest 0-32 bits are the bits
    fn raw_keys(&self) -> Vec<RawKey> {
        // how many bytes are necesary to represent the bits
        let num_bytes = self.bits.num_bits.div_ceil(8) as usize;
        let mut bytes = Vec::with_capacity(num_bytes + 10);
        bytes.extend(self.version.to_be_bytes());
        bytes.extend(self.bits.num_bits.to_be_bytes());
        bytes.extend(&self.bits.bytes[..num_bytes]);
        vec![RawKey::Owned(bytes)]
    }

    fn deserialize(slice: &[u8]) -> StdResult<Self::Output> {
        let range = 10..=(10 + N);
        if !range.contains(&slice.len()) {
            return Err(StdError::deserialize::<Self>(
                format!("slice length must be in the range {range:?}, found {}", slice.len())
            ));
        }

        let (version_bytes, num_bits_bytes, bytes) = (&slice[..2], &slice[2..10], &slice[10..]);
        let version = u64::from_be_bytes(version_bytes.try_into()?);
        let num_bits = u16::from_be_bytes(num_bits_bytes.try_into()?);
        let bytes = bytes.try_into()?;

        Ok(NodeKey {
            version,
            bits: BitArray { num_bits, bytes },
        })
    }
}
