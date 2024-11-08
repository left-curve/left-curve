use {
    crate::{
        Dec128, Dec256, Int128, Int256, Int512, Int64, MathError, MathResult, NumberConst, Udec128,
        Udec256, Uint128, Uint256, Uint512, Uint64,
    },
    bnum::cast::As,
};

pub trait Unsigned {
    type Signed;

    fn checked_into_signed(self) -> MathResult<Self::Signed>;
}

macro_rules! impl_checked_into_signed_std {
    ($unsigned:ty => $signed:ty) => {
        impl Unsigned for $unsigned {
            type Signed = $signed;
            fn checked_into_signed(self) -> MathResult<$signed> {
                if self.0 > <$signed>::MAX.0 as _ {
                    return Err(MathError::overflow_conversion::<$unsigned, $signed>(self));
                }
                Ok(<$signed>::new(self.0 as _))
            }
        }
    };
    ($($unsigned:ty => $signed:ty),+ $(,)?) => {
        $(
            impl_checked_into_signed_std!($unsigned => $signed);
        )+
    };
}

impl_checked_into_signed_std! {
    Uint64  => Int64,
    Uint128 => Int128,
}

macro_rules! impl_checked_into_signed_bnum {
    ($unsigned:ty => $signed:ty) => {
        impl Unsigned for $unsigned {
            type Signed = $signed;
            fn checked_into_signed(self) -> MathResult<$signed> {
                if self.0 > <$signed>::MAX.0.as_() {
                    return Err(MathError::overflow_conversion::<$unsigned, $signed>(self));
                }

                Ok(<$signed>::new(self.0.as_()))
            }
        }
    };
    ($($unsigned:ty => $signed:ty),+ $(,)?) => {
        $(
            impl_checked_into_signed_bnum!($unsigned => $signed);
        )+
    };
}

impl_checked_into_signed_bnum! {
    Uint256 => Int256,
    Uint512 => Int512,
}

macro_rules! impl_chekced_into_signed_dec {
    ($unsigned:ty => $signed:ty) => {
        impl Unsigned for $unsigned {
            type Signed = $signed;
            fn checked_into_signed(self) -> MathResult<$signed> {
                self.0.checked_into_signed().map(<$signed>::raw)
            }
        }
    };
    ($($unsigned:ty => $signed:ty),+ $(,)?) => {
        $(
            impl_chekced_into_signed_dec!($unsigned => $signed);
        )+
    };
}

impl_chekced_into_signed_dec! {
    Udec128 => Dec128,
    Udec256 => Dec256,
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod int_tests {
    use {
        crate::{int_test, test_utils::bt, Int, Int128, Int256, MathError, Unsigned},
        bnum::{cast::As, types::U256},
    };

    int_test!( unsigned_to_signed
        inputs = {
            u128 = {
                passing: [
                    (10u128, Int128::new(10)),
                    (u128::MAX / 2, Int128::new((u128::MAX / 2) as i128)),
                ],
                failing: [
                    u128::MAX / 2 + 1,
                ]
            }
            u256 = {
                passing: [
                    (U256::from(10u128), Int256::new_from_i128(10)),
                    (U256::MAX / 2, Int256::new((U256::MAX / 2).as_())),
                ],
                failing: [
                    U256::MAX / 2 + 1,
                ]
            }
        }
        method = |_0, samples, failing_samples| {
            for (unsigned, expected) in samples {
                let uint = bt(_0, Int::new(unsigned));
                assert_eq!(uint.checked_into_signed().unwrap(), expected);
            }

            for unsigned in failing_samples {
                let uint = bt(_0, Int::new(unsigned));
                assert!(matches!(uint.checked_into_signed(), Err(MathError::OverflowConversion { .. })));
            }
        }
    );
}

#[cfg(test)]
mod dec_tests {
    use {
        crate::{
            dec_test,
            test_utils::{dec, dt},
            Dec, Dec128, Dec256, FixedPoint, Int128, Int256, MathError, NumberConst, Udec256,
            Unsigned,
        },
        bnum::{cast::As, types::U256},
    };

    dec_test!( unsigned_to_signed
        inputs = {
            udec128 = {
                passing: [
                    (Dec::TEN, Dec128::TEN),
                    (Dec::MAX / dec("2"), Dec128::raw(Int128::new((u128::MAX / 2) as i128))),
                ],
                failing: [
                    Dec::MAX / dec("2") + Dec::TICK,
                ]
            }
            udec256 = {
                passing: [
                    (Dec::TEN, Dec256::TEN),
                    (Udec256::MAX / dec("2"), Dec256::raw(Int256::new((U256::MAX / 2).as_()))),
                ],
                failing: [
                    Dec::MAX / dec("2") + Dec::TICK,
                ]
            }
        }
        method = |_0d: Dec<_, 18>, passing, failing| {
            for (unsigned, expected) in passing {
                dt(_0d, unsigned);
                assert_eq!(unsigned.checked_into_signed().unwrap(), expected);
            }

            for unsigned in failing {
                dt(_0d, unsigned);
                assert!(matches!(unsigned.checked_into_signed(), Err(MathError::OverflowConversion { .. })));
            }
        }
    );
}
