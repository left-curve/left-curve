use {
    crate::{forward_ref_partial_eq, StdError, StdResult, Uint256, Uint64},
    forward_ref::{forward_ref_binop, forward_ref_op_assign},
    serde::{de, ser},
    std::{
        fmt,
        ops::{
            Add, AddAssign, Div, DivAssign, Mul, MulAssign, Rem, RemAssign, Shl, ShlAssign, Shr,
            ShrAssign, Sub, SubAssign,
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

forward_ref_partial_eq!(Uint128, Uint128);

impl Uint128 {
    pub const MAX:  Self = Self(u128::MAX);
    pub const MIN:  Self = Self(u128::MIN);
    pub const ZERO: Self = Self(0);
    pub const ONE:  Self = Self(1);

    pub const fn new(value: u128) -> Self {
        Self(value)
    }

    pub const fn u128(self) -> u128 {
        self.0
    }

    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    pub const fn from_be_bytes(data: [u8; 16]) -> Self {
        Self(u128::from_be_bytes(data))
    }

    pub const fn from_le_bytes(data: [u8; 16]) -> Self {
        Self(u128::from_le_bytes(data))
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
            .checked_div(other.0)
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

    pub fn checked_multiply_ratio(self, nominator: Self, denominator: Self) -> StdResult<Self> {
        (Uint256::from(self) * Uint256::from(nominator) / Uint256::from(denominator)).try_into()
    }

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

impl From<u128> for Uint128 {
    fn from(value: u128) -> Self {
        Self(value)
    }
}

impl From<Uint128> for u128 {
    fn from(value: Uint128) -> Self {
        value.0
    }
}

impl From<Uint64> for Uint128 {
    fn from(value: Uint64) -> Self {
        Self(value.u64().into())
    }
}

impl TryFrom<Uint128> for Uint64 {
    type Error = StdError;

    fn try_from(value: Uint128) -> StdResult<Self> {
        value
            .0
            .try_into()
            .map(Uint64::new)
            .map_err(|_| StdError::overflow_conversion::<_, Uint64>(value))
    }
}

impl FromStr for Uint128 {
    type Err = StdError;

    fn from_str(s: &str) -> StdResult<Self> {
        u128::from_str(s)
            .map(Self)
            .map_err(|err| StdError::parse_number::<Self>(s, err))
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
        f.write_str("a string-encoded 128-bit unsigned integer")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        v.parse::<u128>().map(Uint128::new).map_err(E::custom)
    }
}
