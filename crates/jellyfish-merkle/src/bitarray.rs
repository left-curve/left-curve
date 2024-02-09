use {
    cw_std::{Hash, Order},
    std::fmt,
};

#[derive(Clone, PartialEq, Eq)]
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

    pub fn new_empty() -> Self {
        Self {
            num_bits: 0,
            bytes: [0; Self::MAX_BYTE_LENGTH],
        }
    }

    pub fn from_bytes(slice: &[u8]) -> Self {
        // the slice must be no longer than 32 bytes, otherwise panic
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

    /// Iterate the bits in the index range. `min` is inclusive, `max` exclusive.
    /// If min >= max, an empty iterator is returned.
    pub fn range(&self, min: Option<usize>, max: Option<usize>, order: Order) -> BitIterator {
        let min = min.unwrap_or(0);
        let max = max.unwrap_or(self.num_bits);
        BitIterator::new(&self.bytes, min, max, order)
    }
}

impl fmt::Debug for BitArray {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("BitArray(")?;
        for i in 0..self.num_bits {
            f.write_str(&self.bit_at_index(i).to_string())?;
        }
        f.write_str(")")
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

pub struct BitIterator<'a> {
    bytes:   &'a [u8],
    current: Option<(usize, usize)>, // None if `next` hasn't been called for the 1st time yet
    min:     (usize, usize),
    max:     (usize, usize),
    order:   Order,
}

impl<'a> BitIterator<'a> {
    pub fn new(bytes: &'a [u8], min: usize, max: usize, order: Order) -> Self {
        Self {
            current: None,
            min: (min / 8, min % 8),
            max: (max / 8, max % 8),
            bytes,
            order,
        }
    }

    fn increment_quotient_and_remainder(&mut self) -> Option<(usize, usize)> {
        let Some((q, r)) = self.current.as_mut() else {
            // this is the first time `next` is called. in this case, since the
            // minimum bound in inclusive, we simply return the min q and r.
            // make sure to check the max bound.
            if self.min < self.max {
                self.current = Some(self.min);
                return self.current;
            } else {
                return None;
            }
        };

        if *r == 7 {
            *q += 1;
            *r = 0;
        } else {
            *r += 1;
        }

        if (*q, *r) < self.max {
            Some((*q, *r))
        } else {
            None
        }
    }

    fn decrement_quotient_and_remainder(&mut self) -> Option<(usize, usize)> {
        if self.current.is_none() {
            // this is the first time `next` is called. in this case, initialize
            // current q and r. since max bound is exclusive, we don't just return
            // here yet as in `increment`, but instead move on to decrement it.
            self.current = Some(self.max);
        }
        let (q, r) = self.current.as_mut().unwrap();

        if *r == 0 {
            // prevent subtraction underflow
            if *q == 0 {
                return None;
            }
            *q -= 1;
            *r = 7;
        } else {
            *r -= 1;
        }

        if (*q, *r) >= self.min {
            Some((*q, *r))
        } else {
            None
        }
    }
}

impl<'a> Iterator for BitIterator<'a> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        let (q, r) = if self.order == Order::Ascending {
            self.increment_quotient_and_remainder()?
        } else {
            self.decrement_quotient_and_remainder()?
        };

        let byte = self.bytes[q];
        Some((byte >> (7 - r)) & 0b1)
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, proptest::prelude::*};

    fn build_bitarray_from_booleans(bits: &[bool]) -> BitArray {
        let mut bitarray = BitArray::new_empty();
        for bit in bits {
            bitarray.push(if *bit { 1 } else { 0 });
        }
        bitarray
    }

    proptest! {
        /// Generate 256 random bits, push them one-by-one into the BitArray,
        /// then retrieve them one-by-one. The retrieved msut match the original.
        #[test]
        fn pushing_and_getting(
            booleans in prop::collection::vec(any::<bool>(), BitArray::MAX_BIT_LENGTH),
        ) {
            let bits = build_bitarray_from_booleans(&booleans);
            for (bit, boolean) in bits.range(None, None, Order::Ascending).zip(booleans) {
                prop_assert_eq!(boolean, bit == 1);
            }
        }

        #[test]
        fn iterating_no_bounds(
            booleans in prop::collection::vec(any::<bool>(), BitArray::MAX_BIT_LENGTH),
        ) {
            let bits = build_bitarray_from_booleans(&booleans);
            for (bit, boolean) in bits.range(None, None, Order::Ascending).zip(&booleans) {
                prop_assert_eq!(*boolean, bit == 1);
            };
            for (bit, boolean) in bits.range(None, None, Order::Descending).zip(booleans.iter().rev()) {
                prop_assert_eq!(*boolean, bit == 1);
            };
        }

        #[test]
        fn iterating_with_bounds(
            min in 0..=BitArray::MAX_BIT_LENGTH,
            max in 0..=BitArray::MAX_BIT_LENGTH,
            booleans in prop::collection::vec(any::<bool>(), BitArray::MAX_BIT_LENGTH),
        ) {
            let bits = build_bitarray_from_booleans(&booleans);
            if min >= max {
                // in this case, we just assert the iterator is empty and be done
                prop_assert!(bits.range(Some(min), Some(max), Order::Ascending).next().is_none());
                prop_assert!(bits.range(Some(min), Some(max), Order::Descending).next().is_none());
                return Ok(());
            }
            for (bit, boolean) in bits.range(Some(min), Some(max), Order::Ascending).zip(&booleans[min..max]) {
                prop_assert_eq!(*boolean, bit == 1);
            }
            for (bit, boolean) in bits.range(Some(min), Some(max), Order::Descending).zip(booleans[min..max].iter().rev()) {
                prop_assert_eq!(*boolean, bit == 1);
            }
        }
    }
}
