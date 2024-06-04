use std::{
    fmt::Display,
    ops::{Neg, Sub},
    str::FromStr,
};

use borsh::{BorshDeserialize, BorshSerialize};

use crate::{impl_number, Number, NumberConst, StdError, StdResult, Uint128};

#[derive(
    BorshSerialize, BorshDeserialize, Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord,
)]
pub struct Signed<T> {
    pub(crate) inner: T,
    pub(crate) is_positive: bool,
}

// --- Init ---
impl<T> Signed<T> {
    pub const fn new(inner: T, is_positive: bool) -> Self {
        Self { inner, is_positive }
    }

    pub const fn new_positive(inner: T) -> Self {
        Self {
            inner,
            is_positive: true,
        }
    }

    pub const fn new_negative(inner: T) -> Self {
        Self {
            inner,
            is_positive: false,
        }
    }

    pub fn is_positive(self) -> bool {
        self.is_positive
    }
}

// --- Constants ---
impl<T> NumberConst for Signed<T>
where
    T: NumberConst,
{
    const MAX: Self = Self::new_positive(T::MAX);
    const MIN: Self = Self::new_negative(T::MAX);
    const ONE: Self = Self::new_positive(T::ONE);
    const TEN: Self = Self::new_positive(T::TEN);
    const ZERO: Self = Self::new_positive(T::ZERO);
}

// --- Number ---
impl<T> Number for Signed<T>
where
    T: Number + PartialOrd + NumberConst + Copy + Sub<Output = T>,
    Self: Display,
{
    fn checked_add(self, other: Self) -> StdResult<Self> {
        match (self.is_positive, other.is_positive) {
            (true, true) => self.inner.checked_add(other.inner).map(Self::new_positive),
            (true, false) => {
                if self.inner > other.inner {
                    self.inner.checked_sub(other.inner).map(Self::new_positive)
                } else {
                    other.inner.checked_sub(self.inner).map(Self::new_negative)
                }
            },
            (false, true) => {
                if self.inner > other.inner {
                    self.inner.checked_sub(other.inner).map(Self::new_negative)
                } else {
                    other.inner.checked_sub(self.inner).map(Self::new_positive)
                }
            },
            (false, false) => self.inner.checked_add(other.inner).map(Self::new_negative),
        }
    }

    fn checked_sub(self, other: Self) -> StdResult<Self> {
        match (self.is_positive, other.is_positive) {
            (true, true) => {
                if self.inner > other.inner {
                    self.inner.checked_sub(other.inner).map(Self::new_positive)
                } else {
                    other.inner.checked_sub(self.inner).map(Self::new_negative)
                }
            },
            (true, false) => self.inner.checked_add(other.inner).map(Self::new_positive),
            (false, true) => self.inner.checked_add(other.inner).map(Self::new_negative),
            (false, false) => {
                if self.inner > other.inner {
                    self.inner.checked_sub(other.inner).map(Self::new_negative)
                } else {
                    other.inner.checked_sub(self.inner).map(Self::new_positive)
                }
            },
        }
    }

    fn checked_mul(self, other: Self) -> crate::StdResult<Self> {
        self.inner
            .checked_mul(other.inner)
            .map(|inner| Self::new(inner, self.is_positive == other.is_positive))
    }

    fn checked_div(self, other: Self) -> crate::StdResult<Self> {
        self.inner
            .checked_div(other.inner)
            .map(|inner| Self::new(inner, self.is_positive == other.is_positive))
    }

    /// On signed number, the remainder sign is the same as the dividend.
    fn checked_rem(self, other: Self) -> crate::StdResult<Self> {
        self.inner
            .checked_rem(other.inner)
            .map(|val| Self::new(val, self.is_positive))
    }

    fn checked_pow(self, other: u32) -> crate::StdResult<Self> {
        let pow_sign = if other % 2 == 0 {
            true
        } else {
            self.is_positive
        };
        self.inner
            .checked_pow(other)
            .map(|inner| Self::new(inner, pow_sign))
    }

    fn wrapping_add(self, other: Self) -> Self {
        match (self.is_positive, other.is_positive) {
            // + + + => + + | Wrapping is possible
            (true, true) => {
                let result = self.inner.wrapping_add(other.inner);
                // Wrapped occurred
                if result < self.inner {
                    Self::new_negative(T::MAX - result)
                } else {
                    Self::new_positive(result)
                }
            },
            // + + - => + - | Wrapping is not possible
            (true, false) => {
                if self.inner > other.inner {
                    Self::new_positive(self.inner - other.inner)
                } else {
                    Self::new_negative(other.inner - self.inner)
                }
            },
            // - + + => - + | Wrapping is not possible
            (false, true) => {
                if self.inner > other.inner {
                    Self::new_negative(self.inner - other.inner)
                } else {
                    Self::new_positive(other.inner - self.inner)
                }
            },
            // - + - => - - | Wrapping is possible
            (false, false) => {
                let result = self.inner.wrapping_add(other.inner);
                // Wrapped occurred
                if result < self.inner {
                    Self::new_positive(T::MAX - result)
                } else {
                    Self::new_negative(result)
                }
            },
        }
    }

    fn wrapping_sub(self, other: Self) -> Self {
        match (self.is_positive, other.is_positive) {
            // + - + => + - | Wrapping is not possible
            (true, true) => {
                if self.inner > other.inner {
                    Self::new_positive(self.inner - other.inner)
                } else {
                    Self::new_negative(other.inner - self.inner)
                }
            },
            // + - - => + + | Wrapping is possible
            (true, false) => {
                let result = self.inner.wrapping_add(other.inner);
                // Wrapped occurred
                if result < self.inner {
                    Self::new_negative(T::MAX - result)
                } else {
                    Self::new_positive(result)
                }
            },
            // - - + => - - | Wrapping is possible
            (false, true) => {
                let result = self.inner.wrapping_add(other.inner);
                // Wrapped occurred
                if result < self.inner {
                    Self::new_positive(T::MAX - result)
                } else {
                    Self::new_negative(result)
                }
            },
            // - - - => - + | Wrapping is not possible
            (false, false) => {
                if self.inner > other.inner {
                    Self::new_negative(self.inner - other.inner)
                } else {
                    Self::new_positive(other.inner - self.inner)
                }
            },
        }
    }

    fn wrapping_mul(self, other: Self) -> Self {
        let result = self.inner.wrapping_mul(other.inner);
        Self::new(result, self.is_positive == other.is_positive)
    }

    fn wrapping_pow(self, other: u32) -> Self {
        let pow_sign = if other % 2 == 0 {
            true
        } else {
            self.is_positive
        };
        let result = self.inner.wrapping_pow(other);
        Self::new(result, pow_sign)
    }

    fn saturating_add(self, other: Self) -> Self {
        match (self.is_positive, other.is_positive) {
            // + + + => + + | Saturing is possible
            (true, true) => {
                let result = self.inner.saturating_add(other.inner);
                Self::new_positive(result)
            },
            // + + - => + - | Saturing is not possible
            (true, false) => {
                if self.inner > other.inner {
                    Self::new_positive(self.inner - other.inner)
                } else {
                    Self::new_negative(other.inner - self.inner)
                }
            },
            // - + + => - + | Saturing is not possible
            (false, true) => {
                if self.inner > other.inner {
                    Self::new_negative(self.inner - other.inner)
                } else {
                    Self::new_positive(other.inner - self.inner)
                }
            },
            // - + - => - - | Saturing is possible
            (false, false) => {
                let result = self.inner.saturating_add(other.inner);
                Self::new_negative(result)
            },
        }
    }

    fn saturating_sub(self, other: Self) -> Self {
        match (self.is_positive, other.is_positive) {
            // + - + => + - | Saturing is not possible
            (true, true) => {
                if self.inner > other.inner {
                    Self::new_positive(self.inner - other.inner)
                } else {
                    Self::new_negative(other.inner - self.inner)
                }
            },
            // + - - => + + | Saturing is possible
            (true, false) => {
                let result = self.inner.saturating_mul(other.inner);
                Self::new_positive(result)
            },
            // - - + => - - | Saturing is possible
            (false, true) => {
                let result = self.inner.saturating_add(other.inner);
                Self::new_negative(result)
            },
            // - - - => - + | Saturing is not possible
            (false, false) => {
                if self.inner > other.inner {
                    Self::new_negative(self.inner - other.inner)
                } else {
                    Self::new_positive(other.inner - self.inner)
                }
            },
        }
    }

    fn saturating_mul(self, other: Self) -> Self {
        let result = self.inner.saturating_mul(other.inner);
        Self::new(result, self.is_positive == other.is_positive)
    }

    fn saturating_pow(self, other: u32) -> Self {
        let pow_sign = if other % 2 == 0 {
            true
        } else {
            self.is_positive
        };
        let result = self.inner.saturating_pow(other);
        Self::new(result, pow_sign)
    }

    fn abs(self) -> Self {
        Self::new_positive(self.inner)
    }

    fn is_zero(self) -> bool {
        self.inner.is_zero()
    }

    fn checked_sqrt(self) -> crate::StdResult<Self> {
        if !self.is_positive {
            return Err(crate::StdError::negative_sqrt::<Self>(self));
        }

        self.inner.checked_sqrt().map(Self::new_positive)
    }
}

// --- Display ---
impl<T> Display for Signed<T>
where
    T: Number + Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_positive {
            write!(f, "{}", self.inner)
        } else {
            write!(f, "-{}", self.inner)
        }
    }
}

// --- FromStr ---
impl<T> FromStr for Signed<T>
where
    T: FromStr<Err = StdError>,
{
    type Err = StdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with('-') {
            T::from_str(&s[1..]).map(Self::new_positive)
        } else {
            T::from_str(s).map(Self::new_positive)
        }
    }
}

// --- Neg ---
impl<T> Neg for Signed<T> {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(self.inner, !self.is_positive)
    }
}

impl_number!(impl Signed with Add, add for Signed<T> where sub fn checked_add);
impl_number!(impl Signed with Sub, sub for Signed<T> where sub fn checked_sub);
impl_number!(impl Signed with Mul, mul for Signed<T> where sub fn checked_mul);
impl_number!(impl Signed with Div, div for Signed<T> where sub fn checked_div);

pub type Int128 = Signed<Uint128>;

#[cfg(test)]
mod test {
    use crate::{Int128, Number, NumberConst};

    #[test]
    fn t1_ops() {
        let a = Int128::new_positive(10_u128.into());
        let b = Int128::new_positive(20_u128.into());

        assert_eq!(a + b, Int128::new_positive(30_u128.into()));
        assert_eq!(a - b, Int128::new_negative(10_u128.into()));
        assert_eq!(b - a, Int128::new_positive(10_u128.into()));

        assert_eq!(a * b, Int128::new_positive(200_u128.into()));
        assert_eq!(b * a, Int128::new_positive(200_u128.into()));

        let a = Int128::new_negative(10_u128.into());
        let b = Int128::new_positive(20_u128.into());

        assert_eq!(a + b, Int128::new_positive(10_u128.into()));
        assert_eq!(a - b, Int128::new_negative(30_u128.into()));
        assert_eq!(b - a, Int128::new_positive(30_u128.into()));

        assert_eq!(a * b, Int128::new_negative(200_u128.into()));
        assert_eq!(b * a, Int128::new_negative(200_u128.into()));

        let a = Int128::new_negative(10_u128.into());
        let b = Int128::new_negative(20_u128.into());

        assert_eq!(a * b, Int128::new_positive(200_u128.into()));
        assert_eq!(b * a, Int128::new_positive(200_u128.into()));
        assert_eq!(
            a.checked_pow(2).unwrap(),
            Int128::new_positive(100_u128.into())
        );
        assert_eq!(
            a.checked_pow(3).unwrap(),
            Int128::new_negative(1000_u128.into())
        );
    }

    #[test]
    fn t2_wrapping() {
        assert_eq!(Int128::MAX.wrapping_add(Int128::ONE), Int128::MIN);
        assert_eq!(Int128::MIN.wrapping_add(-Int128::ONE), Int128::MAX);
        assert_eq!(Int128::MIN.wrapping_sub(Int128::ONE), Int128::MAX);
        assert_eq!(Int128::MAX.wrapping_sub(-Int128::ONE), Int128::MIN);

        assert_eq!(Int128::MAX.saturating_add(Int128::ONE), Int128::MAX);
        assert_eq!(Int128::MIN.saturating_add(-Int128::ONE), Int128::MIN);
        assert_eq!(Int128::MIN.saturating_sub(Int128::ONE), Int128::MIN);
        assert_eq!(Int128::MAX.saturating_sub(-Int128::ONE), Int128::MAX);

    }
}
