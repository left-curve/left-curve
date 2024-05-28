use std::{fmt::Display, str::FromStr};

use bnum::types::{U256, U512};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{de, ser};

use crate::{
    call_inner, forward_ref_binop_typed, forward_ref_op_assign_typed, generate_grug_number,
    impl_assign, impl_base_ops, impl_next, StdError, StdResult,
};

use super::traits::{Bytable, CheckedOps, GrugNumber, NextNumer};

#[derive(
    BorshSerialize, BorshDeserialize, Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord,
)]
pub struct Uint<U>(U);

impl<U> Uint<U>
where
    U: GrugNumber + PartialEq + Copy + FromStr,
{
    pub const MAX: Self = Self(U::MAX);
    pub const MIN: Self = Self(U::MIN);
    pub const ZERO: Self = Self(U::ZERO);
    pub const ONE: Self = Self(U::ONE);

    pub const fn new(value: U) -> Self {
        Self(value)
    }

    pub const fn number(self) -> U {
        self.0
    }

    pub fn is_zero(self) -> bool {
        self.0 == U::ZERO
    }
}

/// Rappresent the inner type of the [`Uint`]
///
/// This trait is used in [`generate_grug_number!`](crate::generate_grug_number!) to get the inner type of a [`Uint`]
/// and implement the conversion from the inner type to the [`Uint`]
pub trait UintInner {
    type U;
}

impl<U> UintInner for Uint<U> {
    type U = U;
}

// --- Bytable ---
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
}

#[rustfmt::skip]
// --- CheckedOps ---
impl<U> CheckedOps for Uint<U>
where
    U: CheckedOps,
{
    call_inner!(fn checked_add,    field 0, => Result<Self>);
    call_inner!(fn checked_sub,    field 0, => Result<Self>);
    call_inner!(fn checked_mul,    field 0, => Result<Self>);
    call_inner!(fn checked_div,    field 0, => Result<Self>);
    call_inner!(fn checked_rem,    field 0, => Result<Self>);
    call_inner!(fn checked_pow,    arg u32, => Result<Self>);
    call_inner!(fn checked_shl,    arg u32, => Result<Self>);
    call_inner!(fn checked_shr,    arg u32, => Result<Self>);
    call_inner!(fn checked_ilog2,           => StdResult<u32>);
    call_inner!(fn checked_ilog10,          => StdResult<u32>);
    call_inner!(fn wrapping_add,   field 0, => Self);
    call_inner!(fn wrapping_sub,   field 0, => Self);
    call_inner!(fn wrapping_mul,   field 0, => Self);
    call_inner!(fn wrapping_pow,   arg u32, => Self);
    call_inner!(fn saturating_add, field 0, => Self);
    call_inner!(fn saturating_sub, field 0, => Self);
    call_inner!(fn saturating_mul, field 0, => Self);
    call_inner!(fn saturating_pow, arg u32, => Self);
}

// --- NextNumer ---

// full_mull
impl<U> Uint<U>
where
    Uint<U>: NextNumer,
    <Uint<U> as NextNumer>::Next: From<Uint<U>> + CheckedOps,
{
    pub fn checked_full_mul(self, rhs: impl Into<Self>) -> StdResult<<Uint<U> as NextNumer>::Next> {
        <Uint<U> as NextNumer>::Next::from(self)
            .checked_mul(<Uint<U> as NextNumer>::Next::from(rhs.into()))
    }

    pub fn full_mul(self, rhs: impl Into<Self>) -> <Uint<U> as NextNumer>::Next {
        self.checked_full_mul(rhs).unwrap()
    }
}

// multiply_ratio
impl<U> Uint<U>
where
    Uint<U>: NextNumer + Copy,
    <Uint<U> as NextNumer>::Next: From<Uint<U>> + CheckedOps + TryInto<Uint<U>> + ToString + Clone,
{
    pub fn checked_multiply_ratio<A: Into<Self>, B: Into<Self>>(
        self,
        numerator: A,
        denominator: B,
    ) -> StdResult<Self> {
        let numerator: Self = numerator.into();
        let denominator: <Uint<U> as NextNumer>::Next = Into::<Self>::into(denominator).into();

        let next_result = self.checked_full_mul(numerator)?.checked_div(denominator)?;
        next_result
            .clone()
            .try_into()
            .map_err(|_| StdError::overflow_conversion::<_, Self>(next_result))
    }

    pub fn multiply_ratio<A: Into<Self>, B: Into<Self>>(
        self,
        numerator: A,
        denominator: B,
    ) -> Self {
        self.checked_multiply_ratio(numerator, denominator).unwrap()
    }
}

impl_base_ops!(impl<U> Add, add for Uint<U> where sub fn checked_add);
impl_base_ops!(impl<U> Sub, sub for Uint<U> where sub fn checked_sub);
impl_base_ops!(impl<U> Mul, mul for Uint<U> where sub fn checked_mul);
impl_base_ops!(impl<U> Div, div for Uint<U> where sub fn checked_div);
impl_base_ops!(impl<U> Shl, shl for Uint<U> where sub fn checked_shl, u32);
impl_base_ops!(impl<U> Shr, shr for Uint<U> where sub fn checked_shr, u32);

impl_assign!(impl<U> AddAssign, add_assign for Uint<U> where sub fn checked_add);
impl_assign!(impl<U> SubAssign, sub_assign for Uint<U> where sub fn checked_sub);
impl_assign!(impl<U> MulAssign, mul_assign for Uint<U> where sub fn checked_mul);
impl_assign!(impl<U> DivAssign, div_assign for Uint<U> where sub fn checked_div);
impl_assign!(impl<U> ShrAssign, shr_assign for Uint<U> where sub fn checked_shr, u32);
impl_assign!(impl<U> ShlAssign, shl_assign for Uint<U> where sub fn checked_shl, u32);

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

// TODO: Is worth create macros to impl below traits?

impl<U> FromStr for Uint<U>
where
    U: FromStr,
    <U as FromStr>::Err: ToString,
{
    type Err = StdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        U::from_str(s).map(Self).map_err(|err| StdError::parse_number::<Self>(s, err))
    }
}

impl<U> From<Uint<U>> for String
where
    U: std::fmt::Display,
{
    fn from(value: Uint<U>) -> Self {
        value.to_string()
    }
}

impl<U> std::fmt::Display for Uint<U>
where
    U: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0.to_string())
    }
}

impl<U> ser::Serialize for Uint<U>
where
    U: std::fmt::Display,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de, U> de::Deserialize<'de> for Uint<U>
where
    U: Default + FromStr,
    <U as FromStr>::Err: Display,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_str(UintVisitor::<U>::default())
    }
}

#[derive(Default)]
struct UintVisitor<U> {
    _marker: std::marker::PhantomData<U>,
}

impl<'de, U> de::Visitor<'de> for UintVisitor<U>
where
    U: FromStr,
    <U as FromStr>::Err: Display,
{
    type Value = Uint<U>;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // TODO: Change this message in base at the type of U
        f.write_str("a string-encoded 256-bit unsigned integer")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        v.parse::<U>().map(Uint::<U>).map_err(E::custom)
    }
}

use crate::{impl_bytable_std, impl_checked_ops, impl_number_bound, impl_bytable_bnum};

// Uint64
generate_grug_number!(
    name = Uint64,
    inner_type = u64,
    min = u64::MIN,
    max = u64::MAX,
    zero = 0,
    one = 1,
    byte_len = 8,
    impl_bytable = std,
    from = []
);

// Uint128
generate_grug_number!(
    name = Uint128,
    inner_type = u128,
    min = u128::MIN,
    max = u128::MAX,
    zero = 0,
    one = 1,
    byte_len = 16,
    impl_bytable = std,
    from = [Uint64]
);

// Uint256
generate_grug_number!(
    name = Uint256,
    inner_type = U256,
    min = U256::MIN,
    max = U256::MAX,
    zero = U256::ZERO,
    one = U256::ONE,
    byte_len = 32,
    impl_bytable = bnum,
    from = [Uint64, Uint128]
);

// Uint512
generate_grug_number!(
    name = Uint512,
    inner_type = U512,
    min = U512::MIN,
    max = U512::MAX,
    zero = U512::ZERO,
    one = U512::ONE,
    byte_len = 64,
    impl_bytable = bnum,
    from = [Uint256, Uint64, Uint128]
);

// Implementations of [`Next`] has to be done after all the types are defined.
impl_next!(Uint64, Uint128);
impl_next!(Uint128, Uint256);
impl_next!(Uint256, Uint512);
