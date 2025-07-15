use {
    crate::Int,
    bnum::types::{I256, I512, U256, U512},
};

/// Describes a number's associated constants: minimum and maximum; zero, one,
/// and ten.
pub trait NumberConst {
    const MIN: Self;
    const MAX: Self;
    const ONE: Self;
    const TEN: Self;
    const ZERO: Self;
}

// ------------------------------------ int ------------------------------------

impl<U> NumberConst for Int<U>
where
    U: NumberConst,
{
    const MAX: Self = Self(U::MAX);
    const MIN: Self = Self(U::MIN);
    const ONE: Self = Self(U::ONE);
    const TEN: Self = Self(U::TEN);
    const ZERO: Self = Self(U::ZERO);
}

// ------------------------------------ dec ------------------------------------

// Trait auto-impl for all decimals via `generate_decimals` macro.

// ------------------------------ primitive types ------------------------------

macro_rules! impl_number_const {
    ($t:ty, $min:expr, $max:expr, $zero:expr, $one:expr, $ten:expr) => {
        impl NumberConst for $t {
            const MAX: Self = $max;
            const MIN: Self = $min;
            const ONE: Self = $one;
            const TEN: Self = $ten;
            const ZERO: Self = $zero;
        }

        /// A compile-time check to ensure that the constants are of the correct types.
        const _: () = {
            const fn _check_type(_: $t) {}
            _check_type($min);
            _check_type($max);
            _check_type($zero);
            _check_type($one);
            _check_type($ten);
        };
    };
}

impl_number_const! { u8,   u8::MIN,   u8::MAX,   0,          1,         10        }
impl_number_const! { u16,  u16::MIN,  u16::MAX,  0,          1,         10        }
impl_number_const! { u32,  u32::MIN,  u32::MAX,  0,          1,         10        }
impl_number_const! { u64,  u64::MIN,  u64::MAX,  0,          1,         10        }
impl_number_const! { u128, u128::MIN, u128::MAX, 0,          1,         10        }
impl_number_const! { U256, U256::MIN, U256::MAX, U256::ZERO, U256::ONE, U256::TEN }
impl_number_const! { U512, U512::MIN, U512::MAX, U512::ZERO, U512::ONE, U512::TEN }
impl_number_const! { i8,   i8::MIN,   i8::MAX,   0,          1,         10        }
impl_number_const! { i16,  i16::MIN,  i16::MAX,  0,          1,         10        }
impl_number_const! { i32,  i32::MIN,  i32::MAX,  0,          1,         10        }
impl_number_const! { i64,  i64::MIN,  i64::MAX,  0,          1,         10        }
impl_number_const! { i128, i128::MIN, i128::MAX, 0,          1,         10        }
impl_number_const! { I256, I256::MIN, I256::MAX, I256::ZERO, I256::ONE, I256::TEN }
impl_number_const! { I512, I512::MIN, I512::MAX, I512::ZERO, I512::ONE, I512::TEN }
