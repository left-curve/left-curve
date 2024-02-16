use {
    crate::{forward_ref_partial_eq, StdError, StdResult, Uint128},
    bnum::types::U256,
    forward_ref::{forward_ref_binop, forward_ref_op_assign, forward_ref_unop},
    serde::{de, ser},
    std::{
        fmt,
        ops::{
            Add, AddAssign, Div, DivAssign, Mul, MulAssign, Not, Rem, RemAssign, Shl, ShlAssign,
            Shr, ShrAssign, Sub, SubAssign,
        },
        str::FromStr,
    },
};

/// A wrapper of a 256-bit unsigned integer, serialized as a string.
///
/// JSON supports integer numbers in the range of [-(2^53)+1, (2^53)-1].
/// Numbers beyond this range (uint64, uint128...) need to serialize as strings.
/// https://stackoverflow.com/questions/13502398/json-integers-limit-on-size#comment80159722_13502497
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Uint256(U256);

forward_ref_partial_eq!(Uint256, Uint256);

impl Uint256 {
    pub const MAX:  Self = Self(U256::MAX);
    pub const MIN:  Self = Self(U256::MIN);
    pub const ZERO: Self = Self(U256::ZERO);
    pub const ONE:  Self = Self(U256::ONE);

    // unlike Uint64/128, this function is private.
    // we may change the implementation (using a different library than bnum)
    // so we do not expose bnum types in the public API.
    pub(crate) const fn new(value: U256) -> Self {
        Self(value)
    }

    // this function is made private for the same reason as `new`
    pub(crate) fn u256(self) -> U256 {
        self.0
    }

    pub const fn is_zero(self) -> bool {
        self.0.is_zero()
    }

    pub fn checked_add(self, other: Self) -> StdResult<Self> {
        self.0
            .checked_add(other.0)
            .map(Self::new)
            .ok_or_else(|| StdError::overflow_add(self, other))
    }

    pub fn checked_sub(self, other: Self) -> StdResult<Self> {
        self.0
            .checked_sub(other.0)
            .map(Self::new)
            .ok_or_else(|| StdError::overflow_sub(self, other))
    }

    pub fn checked_mul(self, other: Self) -> StdResult<Self> {
        self.0
            .checked_mul(other.0)
            .map(Self::new)
            .ok_or_else(|| StdError::overflow_mul(self, other))
    }

    pub fn checked_div(self, other: Self) -> StdResult<Self> {
        self.0
            .checked_mul(other.0)
            .map(Self::new)
            .ok_or_else(|| StdError::division_by_zero(self))
    }

    pub fn checked_rem(self, other: Self) -> StdResult<Self> {
        self.0
            .checked_rem(other.0)
            .map(Self::new)
            .ok_or_else(|| StdError::remainder_by_zero(self))
    }

    pub fn checked_pow(self, exp: u32) -> StdResult<Self> {
        self.0
            .checked_pow(exp)
            .map(Self::new)
            .ok_or_else(|| StdError::overflow_pow(self, exp))
    }

    pub fn checked_shl(self, rhs: u32) -> StdResult<Self> {
        self.0
            .checked_shl(rhs)
            .map(Self::new)
            .ok_or_else(|| StdError::overflow_shl(self, rhs))
    }

    pub fn checked_shr(self, rhs: u32) -> StdResult<Self> {
        self.0
            .checked_shr(rhs)
            .map(Self::new)
            .ok_or_else(|| StdError::overflow_shr(self, rhs))
    }

    // note: unlike Uint64/128, there is no `checked_multiply_ratio` method for
    // Uint256, because to implement it we need a Uint512 type which we don't
    // have for now.

    /// Return the largest integer `n` such that `n * n <= self`.
    /// In other words, take the square root and _round down_.
    ///
    /// Adapted from `uint` crate:
    /// https://github.com/paritytech/parity-common/blob/uint-v0.9.5/uint/src/uint.rs#L963-L983
    /// which utilizes the method described in:
    /// https://en.wikipedia.org/wiki/Integer_square_root#Using_only_integer_division
    pub fn integer_sqrt(self) -> Self {
        todo!()
    }
}

impl Add for Uint256 {
    type Output = Self;

    fn add(self, rhs: Uint256) -> Self::Output {
        self.checked_add(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl Sub for Uint256 {
    type Output = Self;

    fn sub(self, rhs: Uint256) -> Self::Output {
        self.checked_sub(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl Mul for Uint256 {
    type Output = Self;

    fn mul(self, rhs: Uint256) -> Self::Output {
        self.checked_mul(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl Div for Uint256 {
    type Output = Self;

    fn div(self, rhs: Uint256) -> Self::Output {
        self.checked_div(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl Rem for Uint256 {
    type Output = Self;

    fn rem(self, rhs: Self) -> Self::Output {
        self.checked_rem(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl Shl<u32> for Uint256 {
    type Output = Self;

    fn shl(self, rhs: u32) -> Self::Output {
        self.checked_shl(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl Shr<u32> for Uint256 {
    type Output = Self;

    fn shr(self, rhs: u32) -> Self::Output {
        self.checked_shr(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl AddAssign for Uint256 {
    fn add_assign(&mut self, rhs: Uint256) {
        *self = *self + rhs;
    }
}

impl SubAssign for Uint256 {
    fn sub_assign(&mut self, rhs: Uint256) {
        *self = *self - rhs;
    }
}

impl MulAssign for Uint256 {
    fn mul_assign(&mut self, rhs: Uint256) {
        *self = *self * rhs;
    }
}

impl DivAssign for Uint256 {
    fn div_assign(&mut self, rhs: Uint256) {
        *self = *self / rhs;
    }
}

impl RemAssign for Uint256 {
    fn rem_assign(&mut self, rhs: Self) {
        *self = *self % rhs;
    }
}

impl ShlAssign<u32> for Uint256 {
    fn shl_assign(&mut self, rhs: u32) {
        *self = *self << rhs;
    }
}

impl ShrAssign<u32> for Uint256 {
    fn shr_assign(&mut self, rhs: u32) {
        *self = *self >> rhs;
    }
}

impl Not for Uint256 {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

forward_ref_unop!(impl Not, not for Uint256);

forward_ref_binop!(impl Add, add for Uint256, Uint256);
forward_ref_binop!(impl Sub, sub for Uint256, Uint256);
forward_ref_binop!(impl Mul, mul for Uint256, Uint256);
forward_ref_binop!(impl Div, div for Uint256, Uint256);
forward_ref_binop!(impl Rem, rem for Uint256, Uint256);
forward_ref_binop!(impl Shl, shl for Uint256, u32);
forward_ref_binop!(impl Shr, shr for Uint256, u32);

forward_ref_op_assign!(impl AddAssign, add_assign for Uint256, Uint256);
forward_ref_op_assign!(impl SubAssign, sub_assign for Uint256, Uint256);
forward_ref_op_assign!(impl MulAssign, mul_assign for Uint256, Uint256);
forward_ref_op_assign!(impl DivAssign, div_assign for Uint256, Uint256);
forward_ref_op_assign!(impl RemAssign, rem_assign for Uint256, Uint256);
forward_ref_op_assign!(impl ShlAssign, shl_assign for Uint256, u32);
forward_ref_op_assign!(impl ShrAssign, shr_assign for Uint256, u32);

impl From<Uint128> for Uint256 {
    fn from(value: Uint128) -> Self {
        Self(value.u128().into())
    }
}

impl FromStr for Uint256 {
    type Err = StdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        U256::from_str(s)
            .map(Self)
            .map_err(|err| StdError::parse_number::<Self>(s, err))
    }
}

impl From<Uint256> for String {
    fn from(value: Uint256) -> Self {
        value.to_string()
    }
}

impl fmt::Display for Uint256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.to_string())
    }
}

impl ser::Serialize for Uint256 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> de::Deserialize<'de> for Uint256 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_str(Uint256Visitor)
    }
}

struct Uint256Visitor;

impl<'de> de::Visitor<'de> for Uint256Visitor {
    type Value = Uint256;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("A string-encoded 256-bit unsigned integer")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let number = v.parse::<U256>().map_err(E::custom)?;
        Ok(Uint256::new(number))
    }
}
