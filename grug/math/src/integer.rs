use {
    crate::{Int, MathError, MathResult},
    bnum::types::{I256, I512, U256, U512},
};

/// Describes operations that integer types must implement, which may not be
/// relevant for non-integer types.
pub trait Integer: Sized + Copy {
    fn checked_ilog2(self) -> MathResult<u32>;

    fn checked_ilog10(self) -> MathResult<u32>;

    fn checked_shl(self, other: u32) -> MathResult<Self>;

    fn checked_shr(self, other: u32) -> MathResult<Self>;

    #[inline]
    fn checked_shl_assign(&mut self, other: u32) -> MathResult<()> {
        *self = self.checked_shl(other)?;
        Ok(())
    }

    #[inline]
    fn checked_shr_assign(&mut self, other: u32) -> MathResult<()> {
        *self = self.checked_shr(other)?;
        Ok(())
    }

    fn wrapping_add(self, other: Self) -> Self;

    fn wrapping_sub(self, other: Self) -> Self;

    fn wrapping_mul(self, other: Self) -> Self;

    fn wrapping_pow(self, exp: u32) -> Self;

    #[inline]
    fn wrapping_add_assign(&mut self, other: Self) {
        *self = self.wrapping_add(other);
    }

    #[inline]
    fn wrapping_sub_assign(&mut self, other: Self) {
        *self = self.wrapping_sub(other);
    }

    #[inline]
    fn wrapping_mul_assign(&mut self, other: Self) {
        *self = self.wrapping_mul(other);
    }

    #[inline]
    fn wrapping_pow_assign(&mut self, exp: u32) {
        *self = self.wrapping_pow(exp);
    }
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

    fn wrapping_add(self, other: Self) -> Self {
        Self(self.0.wrapping_add(other.0))
    }

    fn wrapping_sub(self, other: Self) -> Self {
        Self(self.0.wrapping_sub(other.0))
    }

    fn wrapping_mul(self, other: Self) -> Self {
        Self(self.0.wrapping_mul(other.0))
    }

    fn wrapping_pow(self, exp: u32) -> Self {
        Self(self.0.wrapping_pow(exp))
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

            fn wrapping_add(self, other: Self) -> Self {
                self.wrapping_add(other)
            }

            fn wrapping_sub(self, other: Self) -> Self {
                self.wrapping_sub(other)
            }

            fn wrapping_mul(self, other: Self) -> Self {
                self.wrapping_mul(other)
            }

            fn wrapping_pow(self, exp: u32) -> Self {
                self.wrapping_pow(exp)
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
            Bytable, Int, Integer, MathError, Number, NumberConst, Uint128, Uint256, dts, int_test,
            test_utils::bt,
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

    int_test!( wrapping_add
        inputs = {
            u128 = {
                passing: [
                    (Int::MAX, Int::ONE, Int::ZERO)
                ]
            }
            u256 = {
                passing: [
                    (Int::MAX, Int::ONE, Int::ZERO)
                ]
            }
            i128 = {
                passing: [
                    (Int::MAX, Int::ONE, Int::MIN),
                    (Int::MIN, -Int::ONE, Int::MAX),
                ]
            }
            i256 = {
                passing: [
                    (Int::MAX, Int::ONE, Int::MIN),
                    (Int::MIN, -Int::ONE, Int::MAX),
                ]
            }
        }
        method = |_0d: Int<_>, passing| {
            for (left, right, expected) in passing {
                dts!(_0d, left, right, expected);
                assert_eq!(left.wrapping_add(right), expected);
            }
        }
    );

    int_test!( wrapping_sub
        inputs = {
            u128 = {
                passing: [
                    (Int::ZERO, Int::ONE, Int::MAX)
                ]
            }
            u256 = {
                passing: [
                    (Int::ZERO, Int::ONE, Int::MAX)
                ]
            }
            i128 = {
                passing: [
                    (Int::MIN, Int::ONE, Int::MAX),
                    (Int::MAX, -Int::ONE, Int::MIN),
                ]
            }
            i256 = {
                passing: [
                    (Int::MIN, Int::ONE, Int::MAX),
                    (Int::MAX, -Int::ONE, Int::MIN),
                ]
            }
        }
        method = |_0d: Int<_>, passing| {
            for (left, right, expected) in passing {
                dts!(_0d, left, right, expected);
                assert_eq!(left.wrapping_sub(right), expected);
            }
        }
    );

    int_test!( wrapping_mul
        inputs = {
            u128 = {
                passing: [
                    (u128::MAX, 2_u128, u128::MAX - 1),
                    (u128::MAX, 3_u128, u128::MAX - 2),
                ]
            }
            u256 = {
                passing: [
                    (U256::MAX, U256::from(2_u32), U256::MAX - U256::ONE),
                    (U256::MAX, U256::from(3_u32), U256::MAX - U256::from(2_u32)),
                ]
            }
            i128 = {
                passing: [
                    (i128::MAX, 2_i128, -2_i128),
                    (i128::MAX, 3_i128, i128::MAX - 2),
                    (i128::MAX, 4_i128, -4_i128),
                    (i128::MAX, 5_i128, i128::MAX - 4),
                    (i128::MIN, 2_i128, 0),
                    (i128::MIN, 3_i128, i128::MIN),
                    (i128::MIN, 4_i128, 0),
                    (i128::MIN, 5_i128, i128::MIN),
                ]
            }
            i256 = {
                passing: [
                    (I256::MAX, I256::from(2), I256::from(-2)),
                    (I256::MAX, I256::from(3), I256::MAX - I256::from(2)),
                    (I256::MAX, I256::from(4), I256::from(-4)),
                    (I256::MAX, I256::from(5), I256::MAX - I256::from(4)),
                    (I256::MIN, I256::from(2), I256::ZERO),
                    (I256::MIN, I256::from(3), I256::MIN),
                    (I256::MIN, I256::from(4), I256::ZERO),
                    (I256::MIN, I256::from(5), I256::MIN),
                ]
            }
        }
        method = |_0, samples| {
            for (left, right, expected) in samples {
                let left = Int::new(left);
                let right = Int::new(right);
                let expected = Int::new(expected);
                dts!(_0, left, right, expected);
                assert_eq!(left.wrapping_mul(right), expected);
            }
       }
    );

    int_test!( wrapping_pow
        inputs = {
            u128 = {
                passing: [
                    (u128::MAX, 2, 1),
                    (u128::MAX, 3, u128::MAX),
                    (u128::MAX, 4, 1),
                    (u128::MAX, 5, u128::MAX),
                ]
            }
            u256 = {
                passing: [
                    (U256::MAX, 2, U256::ONE),
                    (U256::MAX, 3, U256::MAX),
                    (U256::MAX, 4, U256::ONE),
                    (U256::MAX, 5, U256::MAX),
                ]
            }
            i128 = {
                passing: [
                    (i128::MAX, 2, 1),
                    (i128::MAX, 3, i128::MAX),
                    (i128::MAX, 4, 1),
                    (i128::MAX, 5, i128::MAX),
                    (i128::MIN, 2, 0),
                    (i128::MIN, 3, 0),
                    (i128::MIN, 4, 0),
                ]
            }
            i256 = {
                passing: [
                    (I256::MAX, 2, I256::ONE),
                    (I256::MAX, 3, I256::MAX),
                    (I256::MAX, 4, I256::ONE),
                    (I256::MAX, 5, I256::MAX),
                    (I256::MIN, 2, I256::ZERO),
                    (I256::MIN, 3, I256::ZERO),
                    (I256::MIN, 4, I256::ZERO),
                ]
            }
        }
        method = |_0, samples| {
            for (base, exp, expected) in samples {
                let base = Int::new(base);
                let expected = Int::new(expected);
                dts!(_0, base, expected);
                assert_eq!(base.wrapping_pow(exp), expected);
            }
        }
    );
}
