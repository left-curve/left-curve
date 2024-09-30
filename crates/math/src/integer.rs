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
        crate::{
            int_test, test_utils::bt, Bytable, Int, Integer, MathError, Number, NumberConst,
            Uint128, Uint256,
        },
        bnum::types::{I256, U256},
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

    int_test!( checked_shr
        inputs = {
            u128 = {
                passing: [
                    (160_u128, 1, 80_u128),
                    (160_u128, 2, 40_u128),
                    (160_u128, 3, 20_u128),
                ],
                failing: [
                    128,
                ]
            }
            u256 = {
                passing: [
                    (U256::from(160_u128), 1, U256::from(80_u128)),
                    (U256::from(160_u128), 2, U256::from(40_u128)),
                    (U256::from(160_u128), 3, U256::from(20_u128)),
                ],
                failing: [
                    256,
                ]
            }
            i128 = {
                passing: [
                    (160_i128, 1, 80_i128),
                    (160_i128, 2, 40_i128),
                    (160_i128, 3, 20_i128),
                    (-160_i128, 1, -80_i128),
                    (-160_i128, 2, -40_i128),
                    (-160_i128, 3, -20_i128),
                ],
                failing: [
                    128,
                ]
            }
            i256 = {
                passing: [
                    (I256::from(160_i128), 1, I256::from(80_i128)),
                    (I256::from(160_i128), 2, I256::from(40_i128)),
                    (I256::from(160_i128), 3, I256::from(20_i128)),
                    (I256::from(-160_i128), 1, I256::from(-80_i128)),
                    (I256::from(-160_i128), 2, I256::from(-40_i128)),
                    (I256::from(-160_i128), 3, I256::from(-20_i128)),
                ],
                failing: [
                    256,
                ]
            }
        }
        method = |_0, passing, failing| {
            for (base, shift, expect) in passing {
                let base = Int::new(base);
                let expect = Int::new(expect);
                assert_eq!(base.checked_shr(shift).unwrap(), expect);
            }
            for shift in failing {
                let base = bt(_0, Int::ONE);
                assert!(matches!(base.checked_shr(shift), Err(MathError::OverflowShr { .. })));
            }

        }
    );

    int_test!( shr_panic
        inputs = {
            u128 = [128]
            u256 = [256]
            i128 = [128]
            i256 = [256]
        }
        attrs = #[should_panic(expected = "shift overflow")]
        method = |_0, shift| {
            let base = bt(_0, Int::MAX);
            let _ = base << shift;
        }
    );

    int_test!( checked_shl
        inputs = {
            u128 = {
                passing: [
                    (160_u128, 1, 320_u128),
                    (160_u128, 2, 640_u128),
                    (160_u128, 3, 1280_u128),
                ],
                failing: [
                    128,
                ]
            }
            u256 = {
                passing: [
                    (U256::from(160_u128), 1, U256::from(320_u128)),
                    (U256::from(160_u128), 2, U256::from(640_u128)),
                    (U256::from(160_u128), 3, U256::from(1280_u128)),
                ],
                failing: [
                    256,
                ]
            }
            i128 = {
                passing: [
                    (160_i128, 1, 320_i128),
                    (160_i128, 2, 640_i128),
                    (160_i128, 3, 1280_i128),
                    (-160_i128, 1, -320_i128),
                    (-160_i128, 2, -640_i128),
                    (-160_i128, 3, -1280_i128),
                ],
                failing: [
                    128,
                ]
            }
            i256 = {
                passing: [
                    (I256::from(160_i128), 1, I256::from(320_i128)),
                    (I256::from(160_i128), 2, I256::from(640_i128)),
                    (I256::from(160_i128), 3, I256::from(1280_i128)),
                    (I256::from(-160_i128), 1, I256::from(-320_i128)),
                    (I256::from(-160_i128), 2, I256::from(-640_i128)),
                    (I256::from(-160_i128), 3, I256::from(-1280_i128)),
                ],
                failing: [
                    256,
                ]
            }
        }
        method = |_0, passing, failing| {
            for (base, shift, expect) in passing {
                let base = Int::new(base);
                let expect = Int::new(expect);
                assert_eq!(base.checked_shl(shift).unwrap(), expect);
            }
            for shift in failing {
                let base = bt(_0, Int::MAX);
                assert!(matches!(base.checked_shl(shift), Err(MathError::OverflowShl { .. })));
            }
        }
    );

    int_test!( shl_panic
        inputs = {
            u128 = [128]
            u256 = [256]
            i128 = [128]
            i256 = [256]
        }
        attrs = #[should_panic(expected = "shift overflow")]
        method = |_0, shift| {
            let base = bt(_0, Int::ONE);
            let _ = base << shift;
        }
    );

    int_test!( checked_ilog2
        inputs = {
            u128 = {
                passing: [
                    (1024_u128, 10),
                    (1025_u128, 10),
                    (2047_u128, 10),
                    (2048_u128, 11)
                ],
                failing: []
            }
            u256 = {
                passing: [
                    (U256::from(1024_u128), 10),
                    (U256::from(1025_u128), 10),
                    (U256::from(2047_u128), 10),
                    (U256::from(2048_u128), 11)
                ],
                failing: []
            }
            i128 = {
                passing: [
                    (1024_i128, 10),
                    (1025_i128, 10),
                    (2047_i128, 10),
                    (2048_i128, 11)
                ],
                failing: [
                    -1_i128,
                ]
            }
            i256 = {
                passing: [
                    (I256::from(1024_i128), 10),
                    (I256::from(1025_i128), 10),
                    (I256::from(2047_i128), 10),
                    (I256::from(2048_i128), 11)
                ],
                failing: [
                    -I256::ONE,
                ]
            }
        }
        method = |_0: Int<_>, samples, failing| {
            for (base, expect) in samples {
                let base = Int::new(base);
                assert_eq!(base.checked_ilog2().unwrap(), expect);
            }
            for base in failing {
                let base = bt(_0, Int::new(base));
                assert!(matches!(base.checked_ilog2(), Err(MathError::ZeroLog)));

            }
            // 0 log
            assert!(matches!(_0.checked_ilog2(), Err(MathError::ZeroLog)))
        }
    );

    int_test!( checked_ilog10
        inputs = {
            u128 = {
                passing: [
                    (100_u128, 2),
                    (101_u128, 2),
                    (999_u128, 2),
                    (1000_u128, 3)
                ],
                failing: []
            }
            u256 = {
                passing: [
                    (U256::from(100_u128), 2),
                    (U256::from(101_u128), 2),
                    (U256::from(999_u128), 2),
                    (U256::from(1000_u128), 3)
                ],
                failing: []
            }
            i128 = {
                passing: [
                    (100_i128, 2),
                    (101_i128, 2),
                    (999_i128, 2),
                    (1000_i128, 3)
                ],
                failing: [
                    -1_i128,
                ]
            }
            i256 = {
                passing: [
                    (I256::from(100_i128), 2),
                    (I256::from(101_i128), 2),
                    (I256::from(999_i128), 2),
                    (I256::from(1000_i128), 3)
                ],
                failing: [
                    -I256::ONE,
                ]
            }
        }
        method = |_0: Int<_>, samples, failing| {
            for (base, expect) in samples {
                let base = Int::new(base);
                assert_eq!(base.checked_ilog10().unwrap(), expect);
            }
            for base in failing {
                let base = bt(_0, Int::new(base));
                assert!(matches!(base.checked_ilog10(), Err(MathError::ZeroLog)));

            }
            // 0 log
            assert!(matches!(_0.checked_ilog10(), Err(MathError::ZeroLog)))
        }
    );
}
