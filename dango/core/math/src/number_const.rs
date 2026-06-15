use {
    crate::{Dec, FixedPoint, Int},
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

impl<const S: u32> NumberConst for Dec<u128, S>
where
    Self: FixedPoint<u128>,
{
    const MAX: Self = Self::raw(Int::<u128>::MAX);
    const MIN: Self = Self::raw(Int::<u128>::MIN);
    const ONE: Self = Self::raw(Self::PRECISION);
    const TEN: Self = Self::raw(Int::<u128>::new(u128::TEN.pow(Self::DECIMAL_PLACES + 1)));
    const ZERO: Self = Self::raw(Int::<u128>::ZERO);
}

impl<const S: u32> NumberConst for Dec<U256, S>
where
    Self: FixedPoint<U256>,
{
    const MAX: Self = Self::raw(Int::<U256>::MAX);
    const MIN: Self = Self::raw(Int::<U256>::MIN);
    const ONE: Self = Self::raw(Self::PRECISION);
    const TEN: Self = Self::raw(Int::<U256>::new_from_u128(
        u128::TEN.pow(Self::DECIMAL_PLACES + 1),
    ));
    const ZERO: Self = Self::raw(Int::<U256>::ZERO);
}

impl<const S: u32> NumberConst for Dec<i128, S>
where
    Self: FixedPoint<i128>,
{
    const MAX: Self = Self::raw(Int::<i128>::MAX);
    const MIN: Self = Self::raw(Int::<i128>::MIN);
    const ONE: Self = Self::raw(Self::PRECISION);
    const TEN: Self = Self::raw(Int::<i128>::new(i128::TEN.pow(Self::DECIMAL_PLACES + 1)));
    const ZERO: Self = Self::raw(Int::<i128>::ZERO);
}

impl<const S: u32> NumberConst for Dec<I256, S>
where
    Self: FixedPoint<I256>,
{
    const MAX: Self = Self::raw(Int::<I256>::MAX);
    const MIN: Self = Self::raw(Int::<I256>::MIN);
    const ONE: Self = Self::raw(Self::PRECISION);
    const TEN: Self = Self::raw(Int::<I256>::new_from_i128(
        i128::TEN.pow(Self::DECIMAL_PLACES + 1),
    ));
    const ZERO: Self = Self::raw(Int::<I256>::ZERO);
}

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
impl_number_const! { i8,   i8::MIN,   i8::MAX,   0,          1,         10        }
impl_number_const! { i16,  i16::MIN,  i16::MAX,  0,          1,         10        }
impl_number_const! { i32,  i32::MIN,  i32::MAX,  0,          1,         10        }
impl_number_const! { i64,  i64::MIN,  i64::MAX,  0,          1,         10        }
impl_number_const! { i128, i128::MIN, i128::MAX, 0,          1,         10        }

// bnum 0.14 made ZERO/ONE/TEN `pub(crate)`, so we construct them from LE bytes.
macro_rules! impl_number_const_bnum {
    ($t:ty, $n:literal) => {
        impl NumberConst for $t {
            const MIN: Self = <$t>::MIN;
            const MAX: Self = <$t>::MAX;
            const ZERO: Self = <$t>::from_le_bytes([0; $n]);
            const ONE: Self = {
                let mut bytes = [0u8; $n];
                bytes[0] = 1;
                <$t>::from_le_bytes(bytes)
            };
            const TEN: Self = {
                let mut bytes = [0u8; $n];
                bytes[0] = 10;
                <$t>::from_le_bytes(bytes)
            };
        }
    };
}

impl_number_const_bnum!(U256, 32);
impl_number_const_bnum!(U512, 64);
impl_number_const_bnum!(I256, 32);
impl_number_const_bnum!(I512, 64);
