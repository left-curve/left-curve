use {
    crate::{
        forward_ref_binop_typed, forward_ref_op_assign_typed, generate_uint,
        impl_all_ops_and_assign, impl_assign_integer, impl_assign_number, impl_integer, impl_next,
        impl_number, Bytable, Fraction, Inner, Integer, MultiplyFraction, MultiplyRatio,
        NextNumber, Number, NumberConst, Sign, StdError, StdResult,
    },
    bnum::types::{U256, U512},
    borsh::{BorshDeserialize, BorshSerialize},
    forward_ref::{forward_ref_binop, forward_ref_op_assign},
    serde::{de, ser},
    std::{
        fmt::{self, Display},
        marker::PhantomData,
        ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign},
        str::FromStr,
    },
};

// ------------------------------- generic type --------------------------------

#[derive(
    BorshSerialize, BorshDeserialize, Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord,
)]
pub struct Uint<U>(pub(crate) U);

impl<U> Uint<U> {
    pub const fn new(value: U) -> Self {
        Self(value)
    }

    // TODO: this necessary?
    pub fn new_from(value: impl Into<U>) -> Self {
        Self(value.into())
    }
}

impl<U> Uint<U>
where
    U: Copy,
{
    pub const fn number(&self) -> U {
        self.0
    }
}

impl<U> Inner for Uint<U> {
    type U = U;
}

impl<U> Sign for Uint<U> {
    fn is_negative(&self) -> bool {
        false
    }
}

impl<U> NumberConst for Uint<U>
where
    U: NumberConst,
{
    const MAX: Self = Self(U::MAX);
    const MIN: Self = Self(U::MIN);
    const ONE: Self = Self(U::ONE);
    const TEN: Self = Self(U::TEN);
    const ZERO: Self = Self(U::ZERO);
}

impl<U, const S: usize> Bytable<S> for Uint<U>
where
    U: Bytable<S>,
{
    fn from_be_bytes(data: [u8; S]) -> Self {
        Self(U::from_be_bytes(data))
    }

    fn from_le_bytes(data: [u8; S]) -> Self {
        Self(U::from_le_bytes(data))
    }

    fn to_be_bytes(self) -> [u8; S] {
        self.0.to_be_bytes()
    }

    fn to_le_bytes(self) -> [u8; S] {
        self.0.to_le_bytes()
    }

    fn grow_be_bytes<const INPUT_SIZE: usize>(data: [u8; INPUT_SIZE]) -> [u8; S] {
        U::grow_be_bytes(data)
    }

    fn grow_le_bytes<const INPUT_SIZE: usize>(data: [u8; INPUT_SIZE]) -> [u8; S] {
        U::grow_le_bytes(data)
    }
}

impl<U> Number for Uint<U>
where
    U: Number,
{
    fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    fn abs(self) -> Self {
        // `Uint` represents an unsigned integer, so the absolute value is
        // sipmly itself.
        self
    }

    fn checked_add(self, other: Self) -> StdResult<Self> {
        self.0.checked_add(other.0).map(Self)
    }

    fn checked_sub(self, other: Self) -> StdResult<Self> {
        self.0.checked_sub(other.0).map(Self)
    }

    fn checked_mul(self, other: Self) -> StdResult<Self> {
        self.0.checked_mul(other.0).map(Self)
    }

    fn checked_div(self, other: Self) -> StdResult<Self> {
        self.0.checked_div(other.0).map(Self)
    }

    fn checked_rem(self, other: Self) -> StdResult<Self> {
        self.0.checked_rem(other.0).map(Self)
    }

    fn checked_pow(self, other: u32) -> StdResult<Self> {
        self.0.checked_pow(other).map(Self)
    }

    fn checked_sqrt(self) -> StdResult<Self> {
        self.0.checked_sqrt().map(Self)
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

    fn wrapping_pow(self, other: u32) -> Self {
        Self(self.0.wrapping_pow(other))
    }

    fn saturating_add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }

    fn saturating_mul(self, other: Self) -> Self {
        Self(self.0.saturating_mul(other.0))
    }

    fn saturating_pow(self, other: u32) -> Self {
        Self(self.0.saturating_pow(other))
    }
}

impl<U> Integer for Uint<U>
where
    U: Integer,
{
    fn checked_ilog2(self) -> StdResult<u32> {
        self.0.checked_ilog2()
    }

    fn checked_ilog10(self) -> StdResult<u32> {
        self.0.checked_ilog10()
    }

    fn checked_shl(self, other: u32) -> StdResult<Self> {
        self.0.checked_shl(other).map(Self)
    }

    fn checked_shr(self, other: u32) -> StdResult<Self> {
        self.0.checked_shr(other).map(Self)
    }
}

impl<U> Uint<U>
where
    Uint<U>: NextNumber,
    <Uint<U> as NextNumber>::Next: Number,
{
    pub fn checked_full_mul(
        self,
        rhs: impl Into<Self>,
    ) -> StdResult<<Uint<U> as NextNumber>::Next> {
        let s = self.into_next();
        let r = rhs.into().into_next();
        s.checked_mul(r)
    }
}

impl<U> MultiplyRatio for Uint<U>
where
    Uint<U>: NextNumber + NumberConst + Number + Copy,
    <Uint<U> as NextNumber>::Next: Number + ToString + Clone,
{
    fn checked_multiply_ratio_floor<A: Into<Self>, B: Into<Self>>(
        self,
        numerator: A,
        denominator: B,
    ) -> StdResult<Self> {
        let denominator = denominator.into().into_next();
        let next_result = self.checked_full_mul(numerator)?.checked_div(denominator)?;
        next_result
            .clone()
            .try_into()
            .map_err(|_| StdError::overflow_conversion::<_, Self>(next_result))
    }

    fn checked_multiply_ratio_ceil<A: Into<Self>, B: Into<Self>>(
        self,
        numerator: A,
        denominator: B,
    ) -> StdResult<Self> {
        let numerator: Self = numerator.into();
        let dividend = self.checked_full_mul(numerator)?;
        let floor_result = self.checked_multiply_ratio_floor(numerator, denominator)?;
        let remained = dividend.checked_rem(floor_result.into_next())?;
        if !remained.is_zero() {
            floor_result.checked_add(Self::ONE)
        } else {
            Ok(floor_result)
        }
    }
}

impl<U, AsU, F> MultiplyFraction<F, AsU> for Uint<U>
where
    Uint<U>: NumberConst + MultiplyRatio + From<Uint<AsU>> + ToString,
    F: Number + Fraction<AsU> + Sign + ToString,
{
    fn checked_mul_dec_floor(self, rhs: F) -> StdResult<Self> {
        // If the right hand side is zero, then simply return zero.
        if rhs.is_zero() {
            return Ok(Self::ZERO);
        }
        // The left hand side is `Uint`, a non-negative type, so multiplication
        // with any non-zero negative number goes out of bound.
        if rhs.is_negative() {
            return Err(StdError::negative_mul(self, rhs));
        }
        self.checked_multiply_ratio_floor(rhs.numerator(), F::denominator().into_inner())
    }

    fn checked_mul_dec_ceil(self, rhs: F) -> StdResult<Self> {
        if rhs.is_zero() {
            return Ok(Self::ZERO);
        }
        if rhs.is_negative() {
            return Err(StdError::negative_mul(self, rhs));
        }
        self.checked_multiply_ratio_ceil(rhs.numerator(), F::denominator().into_inner())
    }

    fn checked_div_dec_floor(self, rhs: F) -> StdResult<Self> {
        if rhs.is_negative() {
            return Err(StdError::negative_div(self, rhs));
        }
        self.checked_multiply_ratio_floor(F::denominator().into_inner(), rhs.numerator())
    }

    fn checked_div_dec_ceil(self, rhs: F) -> StdResult<Self> {
        if rhs.is_negative() {
            return Err(StdError::negative_div(self, rhs));
        }
        self.checked_multiply_ratio_ceil(F::denominator().into_inner(), rhs.numerator())
    }
}

impl<U> FromStr for Uint<U>
where
    U: FromStr,
    <U as FromStr>::Err: ToString,
{
    type Err = StdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        U::from_str(s)
            .map(Self)
            .map_err(|err| StdError::parse_number::<Self>(s, err))
    }
}

impl<U> fmt::Display for Uint<U>
where
    U: Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<U> ser::Serialize for Uint<U>
where
    Uint<U>: Display,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de, U> de::Deserialize<'de> for Uint<U>
where
    Uint<U>: FromStr,
    <Uint<U> as FromStr>::Err: Display,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_str(UintVisitor::<U>::new())
    }
}

struct UintVisitor<U> {
    _marker: PhantomData<U>,
}

impl<U> UintVisitor<U> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<'de, U> de::Visitor<'de> for UintVisitor<U>
where
    Uint<U>: FromStr,
    <Uint<U> as FromStr>::Err: Display,
{
    type Value = Uint<U>;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("a string-encoded unsigned integer")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Uint::<U>::from_str(v).map_err(E::custom)
    }
}

impl_number!(impl<U> Add, add for Uint<U> where sub fn checked_add);
impl_number!(impl<U> Sub, sub for Uint<U> where sub fn checked_sub);
impl_number!(impl<U> Mul, mul for Uint<U> where sub fn checked_mul);
impl_number!(impl<U> Div, div for Uint<U> where sub fn checked_div);
impl_integer!(impl<U> Shl, shl for Uint<U> where sub fn checked_shl, u32);
impl_integer!(impl<U> Shr, shr for Uint<U> where sub fn checked_shr, u32);

impl_assign_number!(impl<U> AddAssign, add_assign for Uint<U> where sub fn checked_add);
impl_assign_number!(impl<U> SubAssign, sub_assign for Uint<U> where sub fn checked_sub);
impl_assign_number!(impl<U> MulAssign, mul_assign for Uint<U> where sub fn checked_mul);
impl_assign_number!(impl<U> DivAssign, div_assign for Uint<U> where sub fn checked_div);
impl_assign_integer!(impl<U> ShrAssign, shr_assign for Uint<U> where sub fn checked_shr, u32);
impl_assign_integer!(impl<U> ShlAssign, shl_assign for Uint<U> where sub fn checked_shl, u32);

forward_ref_binop_typed!(impl<U> Add, add for Uint<U>, Uint<U>);
forward_ref_binop_typed!(impl<U> Sub, sub for Uint<U>, Uint<U>);
forward_ref_binop_typed!(impl<U> Mul, mul for Uint<U>, Uint<U>);
forward_ref_binop_typed!(impl<U> Div, div for Uint<U>, Uint<U>);
forward_ref_binop_typed!(impl<U> Rem, rem for Uint<U>, Uint<U>);
forward_ref_binop_typed!(impl<U> Shl, shl for Uint<U>, u32);
forward_ref_binop_typed!(impl<U> Shr, shr for Uint<U>, u32);

forward_ref_op_assign_typed!(impl<U> AddAssign, add_assign for Uint<U>, Uint<U>);
forward_ref_op_assign_typed!(impl<U> SubAssign, sub_assign for Uint<U>, Uint<U>);
forward_ref_op_assign_typed!(impl<U> MulAssign, mul_assign for Uint<U>, Uint<U>);
forward_ref_op_assign_typed!(impl<U> DivAssign, div_assign for Uint<U>, Uint<U>);
forward_ref_op_assign_typed!(impl<U> ShrAssign, shr_assign for Uint<U>, u32);
forward_ref_op_assign_typed!(impl<U> ShlAssign, shl_assign for Uint<U>, u32);

// ------------------------------ concrete types -------------------------------

generate_uint!(
    name = Uint64,
    inner_type = u64,
    from_int = [],
    from_std = [u32, u16, u8],
);

generate_uint!(
    name = Uint128,
    inner_type = u128,
    from_int = [Uint64],
    from_std = [u32, u16, u8],
);

generate_uint!(
    name = Uint256,
    inner_type = U256,
    from_int = [Uint64, Uint128],
    from_std = [u32, u16, u8],
);

generate_uint!(
    name = Uint512,
    inner_type = U512,
    from_int = [Uint256, Uint64, Uint128],
    from_std = [u32, u16, u8],
);

// TODO: can we merge these into `generate_uint`?
impl_next!(Uint64, Uint128);
impl_next!(Uint128, Uint256);
impl_next!(Uint256, Uint512);

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, crate::Dec128};

    /// Make sure we can't multiply a positive integer by a negative decimal.
    #[test]
    fn multiply_fraction_by_negative() {
        let lhs = Uint128::new(123);

        // Multiplying with a negative fraction should fail
        let rhs = Dec128::from_str("-0.1").unwrap();
        assert!(lhs.checked_mul_dec_floor(rhs).is_err());
        assert!(lhs.checked_mul_dec_ceil(rhs).is_err());
        assert!(lhs.checked_div_dec_floor(rhs).is_err());
        assert!(lhs.checked_div_dec_ceil(rhs).is_err());

        // Multiplying with negative zero is allowed though
        let rhs = Dec128::from_str("-0").unwrap();
        assert!(lhs.checked_mul_dec_floor(rhs).unwrap().is_zero());
        assert!(lhs.checked_mul_dec_ceil(rhs).unwrap().is_zero());

        // Dividing by zero should fail
        assert!(lhs.checked_div_dec_floor(rhs).is_err());
        assert!(lhs.checked_div_dec_ceil(rhs).is_err());
    }
}
