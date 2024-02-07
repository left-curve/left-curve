use cw_std::Hash;

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

    pub fn from_bytes(slice: &[u8]) -> Self {
        // the slice must be no longer than 32 bytes, otherwise panic
        let slice = slice.as_ref();
        assert!(slice.len() <= Self::MAX_BYTE_LENGTH, "slice too long: {} > {}", slice.len(), Self::MAX_BYTE_LENGTH);

        // copy the bytes over
        let mut bytes = [0; Self::MAX_BYTE_LENGTH];
        bytes[..slice.len()].copy_from_slice(slice);

        Self {
            num_bits: slice.len() * 8,
            bytes,
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
}

impl From<Hash> for BitArray {
    fn from(hash: Hash) -> Self {
        Self {
            num_bits: Self::MAX_BIT_LENGTH,
            bytes: hash.into_slice(),
        }
    }
}

impl PartialEq<Hash> for BitArray {
    fn eq(&self, hash: &Hash) -> bool {
        self.num_bits == Self::MAX_BIT_LENGTH && self.bytes == hash.as_ref()
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
    }
}
