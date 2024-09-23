use {
    crate::utils::{grow_be_uint, grow_le_uint},
    bnum::types::{I256, I512, U256, U512},
};

/// Describes a number that can be convert to and from raw binary representations.
pub trait Bytable<const S: usize>: Sized {
    const BYTE_LEN: usize = S;

    fn from_be_bytes(data: [u8; S]) -> Self;

    fn from_le_bytes(data: [u8; S]) -> Self;

    fn to_be_bytes(self) -> [u8; S];

    fn to_le_bytes(self) -> [u8; S];

    fn grow_be_bytes<const INPUT_SIZE: usize>(data: [u8; INPUT_SIZE]) -> [u8; S];

    fn grow_le_bytes<const INPUT_SIZE: usize>(data: [u8; INPUT_SIZE]) -> [u8; S];

    fn from_be_bytes_growing<const INPUT_SIZE: usize>(data: [u8; INPUT_SIZE]) -> Self {
        Self::from_be_bytes(Self::grow_be_bytes(data))
    }

    fn from_le_bytes_growing<const INPUT_SIZE: usize>(data: [u8; INPUT_SIZE]) -> Self {
        Self::from_le_bytes(Self::grow_le_bytes(data))
    }
}

// ------------------------------------ std ------------------------------------

macro_rules! impl_bytable_std {
    ($t:ty, $rot:literal) => {
        #[deny(unconditional_recursion)]
        impl Bytable<$rot> for $t {
            fn from_be_bytes(data: [u8; $rot]) -> Self {
                Self::from_be_bytes(data)
            }

            fn from_le_bytes(data: [u8; $rot]) -> Self {
                Self::from_le_bytes(data)
            }

            fn to_be_bytes(self) -> [u8; $rot] {
                self.to_be_bytes()
            }

            fn to_le_bytes(self) -> [u8; $rot] {
                self.to_le_bytes()
            }

            fn grow_be_bytes<const INPUT_SIZE: usize>(data: [u8; INPUT_SIZE]) -> [u8; $rot] {
                grow_be_uint::<INPUT_SIZE, $rot>(data)
            }

            fn grow_le_bytes<const INPUT_SIZE: usize>(data: [u8; INPUT_SIZE]) -> [u8; $rot] {
                grow_le_uint::<INPUT_SIZE, $rot>(data)
            }
        }
    };
}

impl_bytable_std!(u8, 1);
impl_bytable_std!(u16, 2);
impl_bytable_std!(u32, 4);
impl_bytable_std!(u64, 8);
impl_bytable_std!(u128, 16);

impl_bytable_std!(i8, 1);
impl_bytable_std!(i16, 2);
impl_bytable_std!(i32, 4);
impl_bytable_std!(i64, 8);
impl_bytable_std!(i128, 16);

// ----------------------------------- bnum ------------------------------------

macro_rules! impl_bytable_bnum {
    ($t:ty, $rot:literal) => {
        impl Bytable<$rot> for $t {
            fn from_be_bytes(bytes: [u8; $rot]) -> Self {
                Self::from_be_slice(&bytes).unwrap()
            }

            fn from_le_bytes(bytes: [u8; $rot]) -> Self {
                Self::from_le_slice(&bytes).unwrap()
            }

            fn to_be_bytes(self) -> [u8; $rot] {
                self.to_le_bytes()
            }

            fn to_le_bytes(self) -> [u8; $rot] {
                self.to_be_bytes()
            }

            fn grow_be_bytes<const INPUT_SIZE: usize>(data: [u8; INPUT_SIZE]) -> [u8; $rot] {
                grow_be_uint::<INPUT_SIZE, $rot>(data)
            }

            fn grow_le_bytes<const INPUT_SIZE: usize>(data: [u8; INPUT_SIZE]) -> [u8; $rot] {
                grow_le_uint::<INPUT_SIZE, $rot>(data)
            }
        }
    };
}

impl_bytable_bnum!(U256, 32);
impl_bytable_bnum!(U512, 64);
impl_bytable_bnum!(I256, 32);
impl_bytable_bnum!(I512, 64);

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        crate::{Bytable, Uint128, Uint256},
        bnum::types::U256,
        proptest::{array::uniform32, prelude::*},
    };

    proptest! {
        /// Ensure the bytable methods work for `Uint128`.
        #[test]
        fn integer_bytable_works_128(number in any::<u128>()) {
            let number = Uint128::from(number);

            // Convert the number to big endian bytes and back, should get the
            // the same value
            let recovered = Uint128::from_be_bytes(number.to_be_bytes());
            prop_assert_eq!(number, recovered);

            // Same thing for little endian
            let recovered = Uint128::from_le_bytes(number.to_le_bytes());
            prop_assert_eq!(number, recovered);
        }

        /// The same test as above, but for `Uint256`.
        #[test]
        fn integer_bytable_works_256(bytes in uniform32(any::<u8>())) {
            let number = Uint256::from_le_bytes(bytes);

            // Convert the number to big endian bytes and back, should get the
            // the same value
            let recovered = Uint256::from_be_bytes(number.to_be_bytes());
            prop_assert_eq!(number, recovered);

            // Same thing for little endian
            let recovered = Uint256::from_le_bytes(number.to_le_bytes());
            prop_assert_eq!(number, recovered);
        }
    }

    #[test]
    fn byt() {
        let bytes = [0u8; 32];
        let a = U256::from_be_slice(&bytes).unwrap();

        let a = U256::from_be_bytes(bytes);

        println!("{a}");
    }
}
