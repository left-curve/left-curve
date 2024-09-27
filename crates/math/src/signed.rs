use {
    crate::{
        Dec128, Dec256, Int128, Int256, Int512, Int64, MathError, MathResult, Sign, Udec128,
        Udec256, Uint128, Uint256, Uint512, Uint64,
    },
    bnum::cast::As,
};

pub trait Signed {
    type Unsigned;

    fn checked_into_unsigned(self) -> MathResult<Self::Unsigned>;
}

macro_rules! impl_checked_into_unsigned_std {
    ($signed:ty => $unsigned:ty) => {
        impl Signed for $signed {
            type Unsigned = $unsigned;

            fn checked_into_unsigned(self) -> MathResult<$unsigned> {
                if self.is_negative() {
                    return Err(MathError::overflow_conversion::<$signed, $unsigned>(self));
                }

                Ok(<$unsigned>::new(self.0 as _))
            }
        }
    };
    ($($signed:ty => $unsigned:ty),+ $(,)?) => {
        $(
            impl_checked_into_unsigned_std!($signed => $unsigned);
        )+
    };
}

impl_checked_into_unsigned_std! {
    Int64  => Uint64,
    Int128 => Uint128,
}

macro_rules! impl_checked_into_unsigned_bnum {
    ($signed:ty => $unsigned:ty) => {

        impl Signed for $signed {
            type Unsigned = $unsigned;

            fn checked_into_unsigned(self) -> MathResult<$unsigned> {
                if self.is_negative() {
                    return Err(MathError::overflow_conversion::<$signed, $unsigned>(self));
                }

                Ok(<$unsigned>::new(self.0.as_()))
            }
        }
    };
    ($($signed:ty => $unsigned:ty),+ $(,)?) => {
        $(
            impl_checked_into_unsigned_bnum!($signed => $unsigned);
        )+
    };
}

impl_checked_into_unsigned_bnum! {
    Int256 => Uint256,
    Int512 => Uint512,
}

macro_rules! impl_checked_into_unsigned_dec {
    ($signed:ty => $unsigned:ty) => {

        impl Signed for $signed {
            type Unsigned = $unsigned;

            fn checked_into_unsigned(self) -> MathResult<$unsigned> {
                self.0.checked_into_unsigned().map(<$unsigned>::raw)
            }
        }
    };
    ($($signed:ty => $unsigned:ty),+ $(,)?) => {
        $(
            impl_checked_into_unsigned_dec!($signed => $unsigned);
        )+
    };
}

impl_checked_into_unsigned_dec! {
    Dec128 => Udec128,
    Dec256 => Udec256,
}

// ------------------------------------ tests ------------------------------------

#[cfg(test)]
mod tests {
    use {
        crate::{int_test, test_utils::bt, Int, MathError, NumberConst, Signed, Uint128, Uint256},
        bnum::{cast::As, types::I256},
    };

    int_test!( singed_to_unsigned,
        Specific
        i128 = [[ // Passing cases
                    (0, Uint128::ZERO),
                    (10i128, Uint128::TEN),
                    (i128::MAX, Uint128::new(i128::MAX as u128))
                ],
                [ // Failing cases
                    -1,
                    i128::MIN
                ]]

        i256 = [[ // Passing cases
                    (I256::ZERO, Uint256::ZERO),
                    (I256::TEN, Uint256::TEN),
                    (I256::MAX, Uint256::new(I256::MAX.as_()))
                ],
                [ // Failing cases
                    -I256::ONE,
                    I256::MIN
                ]]
        => |_0, samples, failing_samples| {
            for (unsigned, expected) in samples {
                let uint = bt(_0, Int::new(unsigned));
                assert_eq!(uint.checked_into_unsigned().unwrap(), expected);
            }

            for unsigned in failing_samples {
                let uint = bt(_0, Int::new(unsigned));
                assert!(matches!(uint.checked_into_unsigned(), Err(MathError::OverflowConversion { .. })));
            }
        }
    );
}
