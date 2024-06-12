use {
    crate::{
        forward_ref_binop_typed, forward_ref_op_assign_typed, generate_signed,
        impl_all_ops_and_assign, impl_assign_number, impl_number, Decimal128, Decimal256, Inner,
        IntPerDec, MultiplyRatio, Number, NumberConst, Rational, Sign, StdError, StdResult, Uint,
        Uint128, Uint256, Uint64,
    },
    borsh::{BorshDeserialize, BorshSerialize},
    forward_ref::{forward_ref_binop, forward_ref_op_assign},
    serde::{Deserialize, Serialize},
    std::{
        cmp::Ordering,
        fmt::Display,
        ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign},
        str::FromStr,
    },
};

// ------------------------------- generic type --------------------------------

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Default, Debug, Clone, Copy)]
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
}

// --- Sign ---
impl<T> Sign for Signed<T> {
    fn is_positive(&self) -> bool {
        self.is_positive
    }
}

// --- Inner ---
impl<T> Inner for Signed<T> {
    type U = T;
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
    fn is_zero(&self) -> bool {
        self.inner.is_zero()
    }

    fn abs(self) -> Self {
        Self::new_positive(self.inner)
    }

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

    fn checked_sqrt(self) -> crate::StdResult<Self> {
        if !self.is_positive {
            return Err(crate::StdError::negative_sqrt::<Self>(self));
        }

        self.inner.checked_sqrt().map(Self::new_positive)
    }
}

// --- Rational ---
impl<T, AsT> Rational<AsT> for Signed<T>
where
    T: Rational<AsT>,
{
    fn numerator(self) -> Uint<AsT> {
        self.inner.numerator()
    }

    fn denominator() -> Uint<AsT> {
        T::denominator()
    }
}

// --- IntPerDecimal ---
impl<T, AsT, DR> IntPerDec<T, AsT, DR> for Signed<T>
where
    T: MultiplyRatio + From<Uint<AsT>>,
    DR: Rational<AsT> + Sign + Copy,
    AsT: NumberConst + Number,
{
    fn checked_mul_dec_floor(self, rhs: DR) -> StdResult<Self> {
        self.inner
            .checked_multiply_ratio_floor(rhs.numerator(), DR::denominator())
            .map(|res| Self::new(res, self.is_positive == rhs.is_positive()))
    }

    fn checked_mul_dec_ceil(self, rhs: DR) -> StdResult<Self> {
        self.inner
            .checked_multiply_ratio_ceil(rhs.numerator(), DR::denominator())
            .map(|res| Self::new(res, self.is_positive == rhs.is_positive()))
    }

    fn checked_div_dec_floor(self, rhs: DR) -> StdResult<Self> {
        self.inner
            .checked_multiply_ratio_floor(DR::denominator(), rhs.numerator())
            .map(|res| Self::new(res, self.is_positive == rhs.is_positive()))
    }

    fn checked_div_dec_ceil(self, rhs: DR) -> StdResult<Self> {
        self.inner
            .checked_multiply_ratio_ceil(DR::denominator(), rhs.numerator())
            .map(|res| Self::new(res, self.is_positive == rhs.is_positive()))
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
        if let Some(s) = s.strip_prefix('-') {
            T::from_str(s).map(Self::new_negative)
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

// --- PartialEq ---
impl<T> PartialEq for Signed<T>
where
    T: Number + PartialEq + Copy,
{
    fn eq(&self, other: &Self) -> bool {
        if self.inner.is_zero() && other.inner.is_zero() {
            return true;
        }
        self.inner == other.inner && self.is_positive == other.is_positive
    }
}

// --- Eq ---
impl<T> Eq for Signed<T> where T: Number + PartialEq + Copy {}

// --- PartialOrd ---
impl<T> PartialOrd for Signed<T>
where
    T: Ord + Number + Copy,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// --- Ord ---
impl<T> Ord for Signed<T>
where
    T: Ord + Number + Copy,
{
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.is_positive, other.is_positive) {
            (true, true) => self.inner.cmp(&other.inner),
            (true, false) => {
                if self.inner.is_zero() && other.inner.is_zero() {
                    Ordering::Equal
                } else {
                    Ordering::Greater
                }
            },
            (false, true) => {
                if self.inner.is_zero() && other.inner.is_zero() {
                    Ordering::Equal
                } else {
                    Ordering::Less
                }
            },
            (false, false) => other.inner.cmp(&self.inner),
        }
    }
}

impl_number!(impl Signed with Add, add for Signed<T> where sub fn checked_add);
impl_number!(impl Signed with Sub, sub for Signed<T> where sub fn checked_sub);
impl_number!(impl Signed with Mul, mul for Signed<T> where sub fn checked_mul);
impl_number!(impl Signed with Div, div for Signed<T> where sub fn checked_div);

impl_assign_number!(impl Signed with AddAssign, add_assign for Signed<T> where sub fn checked_add);
impl_assign_number!(impl Signed with SubAssign, sub_assign for Signed<T> where sub fn checked_sub);
impl_assign_number!(impl Signed with MulAssign, mul_assign for Signed<T> where sub fn checked_mul);
impl_assign_number!(impl Signed with DivAssign, div_assign for Signed<T> where sub fn checked_div);

forward_ref_binop_typed!(impl<T> Add, add for Signed<T>, Signed<T>);
forward_ref_binop_typed!(impl<T> Sub, sub for Signed<T>, Signed<T>);
forward_ref_binop_typed!(impl<T> Mul, mul for Signed<T>, Signed<T>);
forward_ref_binop_typed!(impl<T> Div, div for Signed<T>, Signed<T>);
forward_ref_binop_typed!(impl<T> Rem, rem for Signed<T>, Signed<T>);

forward_ref_op_assign_typed!(impl<T> AddAssign, add_assign for Signed<T>, Signed<T>);
forward_ref_op_assign_typed!(impl<T> SubAssign, sub_assign for Signed<T>, Signed<T>);
forward_ref_op_assign_typed!(impl<T> MulAssign, mul_assign for Signed<T>, Signed<T>);
forward_ref_op_assign_typed!(impl<T> DivAssign, div_assign for Signed<T>, Signed<T>);

// ------------------------------ concrete types -------------------------------

// Int64
generate_signed!(
    name = Int64,
    inner_type = Uint64,
    from_signed = [],
    from_std = [u8, u16, u32]
);

// Int128
generate_signed!(
    name = Int128,
    inner_type = Uint128,
    from_signed = [Int64],
    from_std = [u8, u16, u32]
);

// Int256
generate_signed!(
    name = Int256,
    inner_type = Uint256,
    from_signed = [Int64, Int128],
    from_std = [u8, u16, u32]
);

// SignedDecimal128
generate_signed!(
    name = SignedDecimal128,
    inner_type = Decimal128,
    from_signed = [],
    from_std = []
);

// SignedDecimal256
generate_signed!(
    name = SignedDecimal256,
    inner_type = Decimal256,
    from_signed = [SignedDecimal128],
    from_std = []
);

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use crate::{
        Decimal128, Int128, IntPerDec, Number, NumberConst, SignedDecimal128, SignedDecimal256,
    };

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

        let a = SignedDecimal128::new_positive(Decimal128::from_str("10").unwrap());
        let b = SignedDecimal128::new_negative(Decimal128::from_str("10").unwrap());

        assert_eq!(a + b, SignedDecimal128::ZERO);
        assert_eq!(a + a, SignedDecimal128::from_str("20").unwrap());
        assert_eq!(b + b, SignedDecimal128::from_str("-20").unwrap());

        assert!(a > b);
        assert!(b < a);
        assert!(
            SignedDecimal128::new_positive(Decimal128::new(0_u128))
                == SignedDecimal128::new_negative(Decimal128::new(0_u128))
        );

        let a = Int128::new_negative(10_u128.into());
        assert_eq!(a + 10_u128, Int128::new_negative(0_u128.into()));
        assert_eq!(a + 10_u64, Int128::new_negative(0_u128.into()));
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

    #[test]
    fn t3_conversion() {
        let foo = SignedDecimal128::from_str("-10.5").unwrap();
        assert_eq!(
            foo,
            SignedDecimal128::new_negative(Decimal128::from_str("10.5").unwrap())
        );
    }

    #[test]
    fn t4_serde() {
        let foo = SignedDecimal128::from_str("-10.5").unwrap();
        let ser = serde_json::to_string(&foo).unwrap();
        let des: SignedDecimal128 = serde_json::from_str(&ser).unwrap();
        assert_eq!(foo, des);

        let foo = SignedDecimal256::from_str("-10.5").unwrap();
        let ser = serde_json::to_string(&foo).unwrap();
        let des: SignedDecimal256 = serde_json::from_str(&ser).unwrap();
        assert_eq!(foo, des);

        let ser = borsh::to_vec(&foo).unwrap();
        let des: SignedDecimal256 = borsh::from_slice(&ser).unwrap();
        assert_eq!(foo, des);
    }

    #[test]
    fn t5_signed_int_per_dec() {
        let foo = Int128::new_negative(10_u128.into());
        let res = foo
            .checked_mul_dec_floor(Decimal128::from_str("2").unwrap())
            .unwrap();

        assert_eq!(res, Int128::new_negative(20_u128.into()));

        let foo = Int128::new_negative(10_u128.into());

        let res = foo
            .checked_mul_dec_floor(SignedDecimal128::from_str("-2").unwrap())
            .unwrap();

        assert_eq!(res, Int128::new_positive(20_u128.into()));
    }
}
