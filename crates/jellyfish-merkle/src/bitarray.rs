use cw_std::{StdError, StdResult};

#[derive(Clone)]
pub struct BitArray<const N: usize = 32> {
    /// For our use case, the maximum size of the bitarray is 256 bits, so the
    /// length can be represented by a u16 (2 bytes).
    pub(crate) num_bits: usize,
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
        debug_assert!(bit == 0 || bit == 1, "bit can only be 0 or 1, got {bit}");
        self.num_bits += 1;
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

        let num_bits = u16::from_be_bytes(slice[..2].try_into()?) as usize;

        let num_bytes = slice.len() - 2;
        let mut bytes = [0; N];
        bytes[..num_bytes].copy_from_slice(&slice[..num_bytes]);

        Ok(BitArray { num_bits, bytes })
    }
}

impl<T: AsRef<[u8]>, const N: usize> From<T> for BitArray<N> {
    fn from(bytes: T) -> Self {
        let bytes = bytes.as_ref();
        assert!(bytes.len() < N);
        Self {
            num_bits: bytes.len() * 8,
            bytes: bytes.try_into().unwrap(),
        }
    }
}
