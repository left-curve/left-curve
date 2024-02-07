use {
    crate::BitArray,
    cw_std::{Hash, MapKey, RawKey, StdError, StdResult},
    std::mem,
};

// we need to serialize NodeKey into binary so that it can be used as keys in
// the backing KV store.
// since we use a 32-byte hash, the NodeKey serializes to 9-41 bytes:
// - the first 8 bytes are the version in big endian
// - the next 2 bytes are the num_bits- in big endian
// - the rest 0-32 bits are the bits
//
// ********|*|********************************
// ^       ^ ^                               ^
// 0       len1,len2                         len3
const LEN_1: usize = mem::size_of::<u64>();         // 8
const LEN_2: usize = LEN_1 + mem::size_of::<u16>(); // 10
const LEN_3: usize = LEN_2 + Hash::LENGTH;          // 41

#[derive(Debug, Clone, PartialEq, Eq)]
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
    ///
    /// Since version and num_bits are of known lengths, and the nodes are stored
    /// under a dedicated namespace, we don't length-prefix them (there's no
    /// worry of clashes of keys).
    fn raw_keys(&self) -> Vec<RawKey> {
        // how many bytes are necesary to represent the bits
        let len = self.bits.num_bits.div_ceil(8);
        let mut bytes = Vec::with_capacity(len + LEN_2);
        bytes.extend(self.version.to_be_bytes());
        // num_bits can be of value 256 at most, so it fits in a u16
        bytes.extend((self.bits.num_bits as u16).to_be_bytes());
        bytes.extend(&self.bits.bytes[..len]);
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
        // copy the bytes over
        let mut bytes = [0u8; BitArray::MAX_BYTE_LENGTH];
        bytes[..slice.len() - LEN_2].copy_from_slice(&slice[LEN_2..]);

        Ok(NodeKey {
            version,
            bits: BitArray { num_bits, bytes },
        })
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, cw_std::MapKey, proptest::prelude::*};

    proptest! {
        /// Generate a random NodeKey (random version, random bitarray of random
        /// length), serialize it to raw bytes, then deserialize it back.
        /// The recovered msut match the original.
        #[test]
        fn serializing_and_deserializing(
            version in 0..u64::MAX,
            bytes in prop::collection::vec(any::<u8>(), 1..=BitArray::MAX_BYTE_LENGTH),
        ) {
            let original = NodeKey {
                version,
                bits: BitArray::from(bytes.as_slice()),
            };

            // serialize
            let original_ref = &original;
            let raw_keys = original_ref.raw_keys();
            prop_assert_eq!(raw_keys.len(), 1);

            // deserialize
            let recovered = <&NodeKey>::deserialize(raw_keys[0].as_ref()).unwrap();
            prop_assert_eq!(original, recovered);
        }
    }
}
