use {
    crate::{MathError, MathResult, NumberConst},
    bnum::types::{U256, U512},
};

/// Describes basic operations that all math types must implement.
pub trait Number: Sized {
    fn is_zero(&self) -> bool;

    fn checked_add(self, other: Self) -> MathResult<Self>;

    fn checked_sub(self, other: Self) -> MathResult<Self>;

    fn checked_mul(self, other: Self) -> MathResult<Self>;

    fn checked_div(self, other: Self) -> MathResult<Self>;

    fn checked_rem(self, other: Self) -> MathResult<Self>;

    fn checked_pow(self, other: u32) -> MathResult<Self>;

    fn checked_sqrt(self) -> MathResult<Self>;

    fn wrapping_add(self, other: Self) -> Self;

    fn wrapping_sub(self, other: Self) -> Self;

    fn wrapping_mul(self, other: Self) -> Self;

    fn wrapping_pow(self, other: u32) -> Self;

    fn saturating_add(self, other: Self) -> Self;

    fn saturating_sub(self, other: Self) -> Self;

    fn saturating_mul(self, other: Self) -> Self;

    fn saturating_pow(self, other: u32) -> Self;
}

macro_rules! impl_number {
    ($t:ty) => {
        impl Number for $t
        where
            $t: NumberConst,
        {
            fn is_zero(&self) -> bool {
                *self == Self::ZERO
            }

            fn checked_add(self, other: Self) -> MathResult<Self> {
                self.checked_add(other)
                    .ok_or_else(|| MathError::overflow_add(self, other))
            }

            fn checked_sub(self, other: Self) -> MathResult<Self> {
                self.checked_sub(other)
                    .ok_or_else(|| MathError::overflow_sub(self, other))
            }

            fn checked_mul(self, other: Self) -> MathResult<Self> {
                self.checked_mul(other)
                    .ok_or_else(|| MathError::overflow_mul(self, other))
            }

            fn checked_div(self, other: Self) -> MathResult<Self> {
                self.checked_div(other)
                    .ok_or_else(|| MathError::division_by_zero(self))
            }

            fn checked_rem(self, other: Self) -> MathResult<Self> {
                self.checked_rem(other)
                    .ok_or_else(|| MathError::division_by_zero(self))
            }

            fn checked_pow(self, other: u32) -> MathResult<Self> {
                self.checked_pow(other)
                    .ok_or_else(|| MathError::overflow_pow(self, other))
            }

            /// Compute a _positive_ integer's _floored_ square root using the
            /// [Babylonian method](https://en.wikipedia.org/wiki/Methods_of_computing_square_roots#Heron's_method).
            fn checked_sqrt(self) -> MathResult<Self> {
                if self.is_zero() {
                    return Ok(Self::ZERO);
                }
                let mut x = self;
                let mut y = (x + 1) >> 1;
                while y < x {
                    x = y;
                    y = (x + self / x) >> 1;
                }
                Ok(x)
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

            fn wrapping_pow(self, other: u32) -> Self {
                self.wrapping_pow(other)
            }

            fn saturating_add(self, other: Self) -> Self {
                self.saturating_add(other)
            }

            fn saturating_sub(self, other: Self) -> Self {
                self.saturating_sub(other)
            }

            fn saturating_mul(self, other: Self) -> Self {
                self.saturating_mul(other)
            }

            fn saturating_pow(self, other: u32) -> Self {
                self.saturating_pow(other)
            }
        }
    };
    ($($t:ty),+ $(,)?) => {
        $(
            impl_number!($t);
        )+
    };
}

impl_number!(u8, u16, u32, u64, u128, U256, U512);
