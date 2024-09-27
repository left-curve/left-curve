use {
    crate::{Int, MathError, MathResult},
    bnum::types::{I256, I512, U256, U512},
};

/// Describes operations that integer types must implement, which may not be
/// relevant for non-integer types.
pub trait Integer: Sized {
    fn checked_ilog2(self) -> MathResult<u32>;

    fn checked_ilog10(self) -> MathResult<u32>;

    fn checked_shl(self, other: u32) -> MathResult<Self>;

    fn checked_shr(self, other: u32) -> MathResult<Self>;
}

// ------------------------------------ int ------------------------------------

impl<U> Integer for Int<U>
where
    U: Integer,
{
    fn checked_ilog2(self) -> MathResult<u32> {
        self.0.checked_ilog2()
    }

    fn checked_ilog10(self) -> MathResult<u32> {
        self.0.checked_ilog10()
    }

    fn checked_shl(self, other: u32) -> MathResult<Self> {
        self.0.checked_shl(other).map(Self)
    }

    fn checked_shr(self, other: u32) -> MathResult<Self> {
        self.0.checked_shr(other).map(Self)
    }
}

// ------------------------------ primitive types ------------------------------

macro_rules! impl_integer {
    ($t:ty) => {
        impl Integer for $t {
            fn checked_shl(self, other: u32) -> MathResult<Self> {
                self.checked_shl(other)
                    .ok_or_else(|| MathError::overflow_shl(self, other))
            }

            fn checked_shr(self, other: u32) -> MathResult<Self> {
                self.checked_shr(other)
                    .ok_or_else(|| MathError::overflow_shr(self, other))
            }

            fn checked_ilog2(self) -> MathResult<u32> {
                self.checked_ilog2().ok_or_else(|| MathError::zero_log())
            }

            fn checked_ilog10(self) -> MathResult<u32> {
                self.checked_ilog10().ok_or_else(|| MathError::zero_log())
            }
        }
    };
    ($($t:ty),+ $(,)?) => {
        $(
            impl_integer!($t);
        )+
    };
}

impl_integer! {
    u8, u16, u32, u64, u128, U256, U512,
    i8, i16, i32, i64, i128, I256, I512,
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        crate::{Bytable, Number, NumberConst, Uint128, Uint256},
        proptest::{array::uniform32, prelude::*},
    };

    proptest! {
        /// Ensure the `checked_sqrt` method works for `Uint128`.
        ///
        /// Since our square root method returns the _floored_ result, we make
        /// sure that:
        /// - `root ** 2 <= square`
        /// - `(root + 1) ** 2 > square`
        #[test]
        fn integer_sqrt_works_128(square in any::<u128>()) {
            let square = Uint128::new(square);
            let root = square.checked_sqrt().unwrap();
            prop_assert!(root * root <= square);
            prop_assert!((root + Uint128::ONE) * (root + Uint128::ONE) > square);
        }

        /// The same test as above, but for `Uint256`.
        #[test]
        fn integer_sqrt_works_256(bytes in uniform32(any::<u8>())) {
            let square = Uint256::from_le_bytes(bytes);
            let root = square.checked_sqrt().unwrap();
            prop_assert!(root * root <= square);
            prop_assert!((root + Uint256::ONE) * (root + Uint256::ONE) > square);
        }
    }
}
