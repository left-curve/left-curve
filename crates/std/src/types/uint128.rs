use {
    crate::{StdError, StdResult},
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

/// A wrapper of uint128, serialized as a string.
///
/// JSON supports integer numbers in the range of [-(2^53)+1, (2^53)-1].
/// Numbers beyond this range (uint64, uint128...) need to serialize as strings.
/// https://stackoverflow.com/questions/13502398/json-integers-limit-on-size#comment80159722_13502497
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Uint128(u128);

impl Uint128 {
    pub const MAX: Self = Self::new(u128::MAX);
    pub const MIN: Self = Self::new(u128::MAX);

    pub const fn new(value: u128) -> Self {
        Self(value)
    }

    pub fn u128(self) -> u128 {
        self.0
    }

    #[inline]
    pub const fn zero() -> Self {
        Self(0)
    }

    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    pub const fn to_be_bytes(self) -> [u8; 16] {
        self.0.to_be_bytes()
    }

    pub const fn to_le_bytes(self) -> [u8; 16] {
        self.0.to_le_bytes()
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

    pub fn checked_multiply_ratio(self, _other: Self) -> StdResult<Self> {
        // need Uint256 implemented
        todo!()
    }
}

impl Add for Uint128 {
    type Output = Self;

    fn add(self, rhs: Uint128) -> Self::Output {
        self.checked_add(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl Sub for Uint128 {
    type Output = Self;

    fn sub(self, rhs: Uint128) -> Self::Output {
        self.checked_sub(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl Mul for Uint128 {
    type Output = Self;

    fn mul(self, rhs: Uint128) -> Self::Output {
        self.checked_mul(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl Div for Uint128 {
    type Output = Self;

    fn div(self, rhs: Uint128) -> Self::Output {
        self.checked_div(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl Rem for Uint128 {
    type Output = Self;

    fn rem(self, rhs: Self) -> Self::Output {
        self.checked_rem(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl Shl<u32> for Uint128 {
    type Output = Self;

    fn shl(self, rhs: u32) -> Self::Output {
        self.checked_shl(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl Shr<u32> for Uint128 {
    type Output = Self;

    fn shr(self, rhs: u32) -> Self::Output {
        self.checked_shr(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl AddAssign for Uint128 {
    fn add_assign(&mut self, rhs: Uint128) {
        *self = *self + rhs;
    }
}

impl SubAssign for Uint128 {
    fn sub_assign(&mut self, rhs: Uint128) {
        *self = *self - rhs;
    }
}

impl MulAssign for Uint128 {
    fn mul_assign(&mut self, rhs: Uint128) {
        *self = *self * rhs;
    }
}

impl DivAssign for Uint128 {
    fn div_assign(&mut self, rhs: Uint128) {
        *self = *self / rhs;
    }
}

impl RemAssign for Uint128 {
    fn rem_assign(&mut self, rhs: Self) {
        *self = *self % rhs;
    }
}

impl ShlAssign<u32> for Uint128 {
    fn shl_assign(&mut self, rhs: u32) {
        *self = *self << rhs;
    }
}

impl ShrAssign<u32> for Uint128 {
    fn shr_assign(&mut self, rhs: u32) {
        *self = *self >> rhs;
    }
}

impl Not for Uint128 {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

forward_ref_unop!(impl Not, not for Uint128);

forward_ref_binop!(impl Add, add for Uint128, Uint128);
forward_ref_binop!(impl Sub, sub for Uint128, Uint128);
forward_ref_binop!(impl Mul, mul for Uint128, Uint128);
forward_ref_binop!(impl Div, div for Uint128, Uint128);
forward_ref_binop!(impl Rem, rem for Uint128, Uint128);
forward_ref_binop!(impl Shl, shl for Uint128, u32);
forward_ref_binop!(impl Shr, shr for Uint128, u32);

forward_ref_op_assign!(impl AddAssign, add_assign for Uint128, Uint128);
forward_ref_op_assign!(impl SubAssign, sub_assign for Uint128, Uint128);
forward_ref_op_assign!(impl MulAssign, mul_assign for Uint128, Uint128);
forward_ref_op_assign!(impl DivAssign, div_assign for Uint128, Uint128);
forward_ref_op_assign!(impl RemAssign, rem_assign for Uint128, Uint128);
forward_ref_op_assign!(impl ShlAssign, shl_assign for Uint128, u32);
forward_ref_op_assign!(impl ShrAssign, shr_assign for Uint128, u32);

impl FromStr for Uint128 {
    type Err = StdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let inner = u128::from_str(s)?;
        Ok(Self::new(inner))
    }
}

impl From<Uint128> for String {
    fn from(value: Uint128) -> Self {
        value.to_string()
    }
}

impl fmt::Display for Uint128 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.to_string())
    }
}

impl ser::Serialize for Uint128 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> de::Deserialize<'de> for Uint128 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_str(Uint128Visitor)
    }
}

struct Uint128Visitor;

impl<'de> de::Visitor<'de> for Uint128Visitor {
    type Value = Uint128;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("A string-encoded 128-bit unsigned integer")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let number = v.parse::<u128>().map_err(E::custom)?;
        Ok(Uint128::new(number))
    }
}
