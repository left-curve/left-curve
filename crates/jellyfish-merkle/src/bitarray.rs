use cw_std::{StdError, StdResult};

pub struct BitArray<const N: usize = 32> {
    /// For our use case, the maximum size of the bitarray is 256 bits, so the
    /// length can be represented by a u16 (2 bytes).
    pub(crate) num_bits: u16,
    /// We opt for the stack-allocated `[u8; N]` over heap-allocated `Vec<u8>`.
    /// In practice, the vast majority of node keys are not the full 256 bits,
    /// so this is a waste of memory space. Essentially, we trade memory usage
    /// for speed.
    /// For blockchain nodes in general, memory is cheap, while time is expensive.
    pub(crate) bytes: [u8; N],
}

impl<const N: usize> BitArray<N> {
    pub fn empty() -> Self {
        Self {
            num_bits: 0,
            bytes: [0; N],
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let num_bytes = self.num_bits.div_ceil(8) as usize;
        let mut bytes = Vec::with_capacity(2 + num_bytes);
        bytes.extend(self.num_bits.to_be_bytes());
        bytes.extend(&self.bytes[..num_bytes]);
        bytes
    }

    pub fn deserialize(slice: &[u8]) -> StdResult<Self> {
        // the length of the bytes should be between 2 and 2+N (inclusive)
        let range = 2..=(2 + N);
        if !range.contains(&slice.len()) {
            return Err(StdError::deserialize::<Self>(
                format!("slice length must be in the range {range:?}, found {}", slice.len())
            ));
        }

        let num_bits = u16::from_be_bytes(slice[..2].try_into()?);

        let num_bytes = slice.len() - 2;
        let mut bytes = [0; N];
        bytes[..num_bytes].copy_from_slice(&slice[..num_bytes]);

        Ok(BitArray { num_bits, bytes })
    }
}

impl<const N: usize> From<&[u8]> for BitArray<N> {
    fn from(bytes: &[u8]) -> Self {
        assert!(bytes.len() < N);
        Self {
            num_bits: (bytes.len() * 8) as u16,
            bytes: bytes.try_into().unwrap(),
        }
    }
}
