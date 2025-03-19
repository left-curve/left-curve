use {
    crate::{
        Int,
        utils::{grow_be_int, grow_be_uint, grow_le_int, grow_le_uint},
    },
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

// ----------------------------------- uint ------------------------------------

impl<U, const S: usize> Bytable<S> for Int<U>
where
    U: Bytable<S>,
{
    fn from_be_bytes(data: [u8; S]) -> Self {
        Self(U::from_be_bytes(data))
    }

    fn from_le_bytes(data: [u8; S]) -> Self {
        Self(U::from_le_bytes(data))
    }

    fn to_be_bytes(self) -> [u8; S] {
        self.0.to_be_bytes()
    }

    fn to_le_bytes(self) -> [u8; S] {
        self.0.to_le_bytes()
    }

    fn grow_be_bytes<const INPUT_SIZE: usize>(data: [u8; INPUT_SIZE]) -> [u8; S] {
        U::grow_be_bytes(data)
    }

    fn grow_le_bytes<const INPUT_SIZE: usize>(data: [u8; INPUT_SIZE]) -> [u8; S] {
        U::grow_le_bytes(data)
    }
}

// ------------------------------------ dec ------------------------------------

// TODO: Bytable for `Dec`

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

macro_rules! impl_bytable_signed_std {
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
                grow_be_int::<INPUT_SIZE, $rot>(data)
            }

            fn grow_le_bytes<const INPUT_SIZE: usize>(data: [u8; INPUT_SIZE]) -> [u8; $rot] {
                grow_le_int::<INPUT_SIZE, $rot>(data)
            }
        }
    };
}

impl_bytable_std!(u8, 1);
impl_bytable_std!(u16, 2);
impl_bytable_std!(u32, 4);
impl_bytable_std!(u64, 8);
impl_bytable_std!(u128, 16);

impl_bytable_signed_std!(i8, 1);
impl_bytable_signed_std!(i16, 2);
impl_bytable_signed_std!(i32, 4);
impl_bytable_signed_std!(i64, 8);
impl_bytable_signed_std!(i128, 16);

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
                let words = self.digits();
                let mut bytes = [[0u8; 8]; $rot / 8];
                for i in 0..$rot / 8 {
                    bytes[i] = words[$rot / 8 - i - 1].to_be_bytes();
                }

                unsafe { std::mem::transmute(bytes) }
            }

            fn to_le_bytes(self) -> [u8; $rot] {
                let words = self.digits();
                let mut bytes = [[0u8; 8]; $rot / 8];
                for i in 0..$rot / 8 {
                    bytes[i] = words[i].to_le_bytes();
                }

                unsafe { std::mem::transmute(bytes) }
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

macro_rules! impl_bytable_signed_bnum {
    ($t:ty, $rot:literal) => {
        impl Bytable<$rot> for $t {
            fn from_be_bytes(bytes: [u8; $rot]) -> Self {
                Self::from_be_slice(&bytes).unwrap()
            }

            fn from_le_bytes(bytes: [u8; $rot]) -> Self {
                Self::from_le_slice(&bytes).unwrap()
            }

            fn to_be_bytes(self) -> [u8; $rot] {
                self.to_bits().to_be_bytes()
            }

            fn to_le_bytes(self) -> [u8; $rot] {
                self.to_bits().to_le_bytes()
            }

            fn grow_be_bytes<const INPUT_SIZE: usize>(data: [u8; INPUT_SIZE]) -> [u8; $rot] {
                grow_be_int::<INPUT_SIZE, $rot>(data)
            }

            fn grow_le_bytes<const INPUT_SIZE: usize>(data: [u8; INPUT_SIZE]) -> [u8; $rot] {
                grow_le_int::<INPUT_SIZE, $rot>(data)
            }
        }
    };
}

impl_bytable_bnum!(U256, 32);
impl_bytable_bnum!(U512, 64);
impl_bytable_signed_bnum!(I256, 32);
impl_bytable_signed_bnum!(I512, 64);

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        crate::{Bytable, Int128, Int256, Uint128, Uint256},
        bnum::types::{I256, I512, U256, U512},
        proptest::{array::uniform32, prelude::*},
    };

    proptest! {
        /// Ensure the bytable methods work for `Uint128`.
        #[test]
        fn integer_bytable_works_u128(number in any::<u128>()) {
            let number = Uint128::new(number);

            // Convert the number to big endian bytes and back, should get the
            // the same value
            let recovered = Uint128::from_be_bytes(number.to_be_bytes());
            prop_assert_eq!(number, recovered);

            // Same thing for little endian
            let recovered = Uint128::from_le_bytes(number.to_le_bytes());
            prop_assert_eq!(number, recovered);
        }

        /// Ensure the bytable methods work for `Uint256`.
        #[test]
        fn integer_bytable_works_u256(bytes in uniform32(any::<u8>())) {
            let number = Uint256::from_le_bytes(bytes);

            // Convert the number to big endian bytes and back, should get the
            // the same value
            let recovered = Uint256::from_be_bytes(number.to_be_bytes());
            prop_assert_eq!(number, recovered);

            // Same thing for little endian
            let recovered = Uint256::from_le_bytes(number.to_le_bytes());
            prop_assert_eq!(number, recovered);
        }

        /// Ensure the bytable methods work for `Int128`.
        #[test]
        fn integer_bytable_works_i128(number in any::<i128>()) {
            let number = Int128::new(number);

            // Convert the number to big endian bytes and back, should get the
            // the same value
            let recovered = Int128::from_be_bytes(number.to_be_bytes());
            prop_assert_eq!(number, recovered);

            // Same thing for little endian
            let recovered = Int128::from_le_bytes(number.to_le_bytes());
            prop_assert_eq!(number, recovered);
        }

        /// Ensure the bytable methods work for `Int256`.
        #[test]
        fn integer_bytable_works_i256(number in uniform32(any::<u8>())) {
            let number = Int256::from_le_bytes(number);

            // Convert the number to big endian bytes and back, should get the
            // the same value
            let recovered = Int256::from_be_bytes(number.to_be_bytes());
            prop_assert_eq!(number, recovered);

            // Same thing for little endian
            let recovered = Int256::from_le_bytes(number.to_le_bytes());
            prop_assert_eq!(number, recovered);
        }

        /// Ensure the grown methods work for `signed`.
        #[test]
        fn grown_signed(number in any::<(i8, i64)>()) {
            macro_rules! test {
                ($t:ty, $($val:expr),+) => {
                    $(
                        let compare = <$t>::from_be_bytes_growing($val.to_be_bytes());
                        assert_eq!(<$t>::from($val), compare);

                        let compare = <$t>::from_le_bytes_growing($val.to_le_bytes());
                        assert_eq!(<$t>::from($val), compare);
                    )+
                };
            }

            test!(i128, number.0, number.1);
            test!(I256, number.0, number.1);
            test!(I512, number.0, number.1);
        }

        /// Ensure the grown methods work for `unsigned`.
        #[test]
        fn grown_unsigned(number in any::<(u32, u64)>()) {
            macro_rules! test {
                ($t:ty, $($val:expr),+) => {
                    $(
                        let compare = <$t>::from_be_bytes_growing($val.to_be_bytes());
                        assert_eq!(<$t>::from($val), compare);

                        let compare = <$t>::from_le_bytes_growing($val.to_le_bytes());
                        assert_eq!(<$t>::from($val), compare);
                    )+
                };
            }

            test!(u128, number.0, number.1);
            test!(U256, number.0, number.1);
            test!(U512, number.0, number.1);
        }

    }
}
