use cw_std::{Hash, StdError, StdResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitArray {
    pub(crate) num_bits: usize,
    /// We opt for the stack-allocated `[u8; N]` over heap-allocated `Vec<u8>`.
    /// In practice, the vast majority of node keys are not the full 256 bits,
    /// so this is a waste of memory space. Essentially, we trade memory usage
    /// for speed.
    /// For blockchain nodes in general, memory is cheap, while time is expensive.
    pub(crate) bytes: [u8; Self::MAX_BYTE_LENGTH],
}

impl BitArray {
    pub const MAX_BIT_LENGTH:  usize = Self::MAX_BYTE_LENGTH * 8; // 256
    pub const MAX_BYTE_LENGTH: usize = Hash::LENGTH;              // 32

    pub fn empty() -> Self {
        Self {
            num_bits: 0,
            bytes: [0; Self::MAX_BYTE_LENGTH],
        }
    }

    // we can't use Rust's `Index` trait, because it requires returning a &u8,
    // so we get a "cannot return local reference" error.
    pub fn bit_at_index(&self, index: usize) -> u8 {
        debug_assert!(index < self.num_bits, "index out of bounds: {index} >= {}", self.num_bits);
        // we can use the `div_rem` method provided by the num-integer crate,
        // not sure if it's more efficient:
        // https://docs.rs/num-integer/latest/num_integer/fn.div_rem.html
        let (quotient, remainder) = (index / 8, index % 8);
        let byte = self.bytes[quotient];
        (byte >> (7 - remainder)) & 0b1
    }

    pub fn push(&mut self, bit: u8) {
        debug_assert!(self.num_bits <= Self::MAX_BIT_LENGTH, "bitarray getting too long");
        debug_assert!(bit == 0 || bit == 1, "bit can only be 0 or 1, got {bit}");
        let (quotient, remainder) = (self.num_bits / 8, self.num_bits % 8);
        let byte = &mut self.bytes[quotient];
        if bit == 1 {
            *byte |= 0b1 << (7 - remainder);
        } else {
            // note: the exclamation mark `!` here is the bitwise NOT operator,
            // not the logical NOT operator.
            // in Rust, `u8` has `!` as bitwise NOT; it doesn't have logical NOT.
            // for comparison, in C, `~` is bitwise NOT and `!` is logical NOT.
            *byte &= !(0b1 << (7 - remainder));
        }
        self.num_bits += 1;
    }

    pub fn serialize(&self) -> Vec<u8> {
        let num_bytes = self.num_bits.div_ceil(8);
        let mut bytes = Vec::with_capacity(1 + num_bytes);
        // num_bits can be of value 256 at most, so (num_bits - 1) fits in a u8
        bytes.extend(((self.num_bits - 1) as u8).to_be_bytes());
        bytes.extend(&self.bytes[..num_bytes]);
        bytes
    }

    pub fn deserialize(slice: &[u8]) -> StdResult<Self> {
        // the length of the bytes should be between 2 and 2+N (inclusive)
        let range = 1..=(1 + Self::MAX_BYTE_LENGTH);
        if !range.contains(&slice.len()) {
            return Err(StdError::deserialize::<Self>(
                format!("slice length must be in the range {range:?}, found {}", slice.len())
            ));
        }

        // we subtracted 1 when serializng, so adding 1 back here
        let num_bits = u8::from_be_bytes(slice[..1].try_into()?) as usize + 1;

        // copy the bytes over
        let num_bytes = slice.len() - 1;
        let mut bytes = [0; Self::MAX_BYTE_LENGTH];
        bytes[..num_bytes].copy_from_slice(&slice[1..=num_bytes]);

        Ok(BitArray { num_bits, bytes })
    }
}

impl<T: AsRef<[u8]>> From<T> for BitArray {
    fn from(slice: T) -> Self {
        // the slice must be no longer than 32 bytes, otherwise panic
        let slice = slice.as_ref();
        let slice_len = slice.len();
        assert!(slice_len <= Self::MAX_BYTE_LENGTH, "slice too long: {slice_len} > {}", Self::MAX_BYTE_LENGTH);

        // copy the bytes over
        let mut bytes = [0; Self::MAX_BYTE_LENGTH];
        (&mut bytes[..slice_len]).copy_from_slice(slice);

        Self {
            num_bits: slice.len() * 8,
            bytes,
        }
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, proptest::prelude::*};

    proptest! {
        /// Generate 256 random bits, push them one-by-one into the BitArray,
        /// then retrieve them one-by-one. The retrieved msut match the original.
        #[test]
        fn pushing_and_getting(bits in prop::collection::vec(any::<bool>(), BitArray::MAX_BIT_LENGTH)) {
            let mut bitarray = BitArray::empty();
            for bit in &bits {
                bitarray.push(if *bit { 1 } else { 0 });
            }
            for (index, bit) in bits.into_iter().enumerate() {
                prop_assert_eq!(bit, bitarray.bit_at_index(index) == 1);
            }
        }

        /// Generate a BitArray of random length, serialize it to raw bytes,
        /// then deserialize it back. The recovered msut match the original.
        #[test]
        fn serializing_and_deserializing(bytes in prop::collection::vec(any::<u8>(), 1..=BitArray::MAX_BYTE_LENGTH)) {
            let original = BitArray::from(bytes.as_slice());
            let recovered = BitArray::deserialize(&original.serialize()).unwrap();
            prop_assert_eq!(original, recovered);
        }
    }
}
