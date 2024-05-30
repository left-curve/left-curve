use {
    crate::{forward_ref_partial_eq, StdError, StdResult, Uint128, Uint512},
    bnum::types::U256,
    forward_ref::{forward_ref_binop, forward_ref_op_assign},
    serde::{de, ser},
    std::{
        fmt, mem,
        ops::{
            Add, AddAssign, Div, DivAssign, Mul, MulAssign, Rem, RemAssign, Shl, ShlAssign, Shr,
            ShrAssign, Sub, SubAssign,
        },
        str::FromStr,
    },
};

/// A wrapper of a 256-bit unsigned integer, serialized as a string.
///
/// JSON supports integer numbers in the range of [-(2^53)+1, (2^53)-1].
/// Numbers beyond this range (uint64, uint128...) need to serialize as strings.
/// <https://stackoverflow.com/questions/13502398/json-integers-limit-on-size#comment80159722_13502497>
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Uint256(pub(crate) U256);

forward_ref_partial_eq!(Uint256, Uint256);

impl Uint256 {
    pub const MAX: Self = Self(U256::MAX);
    pub const MIN: Self = Self(U256::MIN);
    pub const ONE: Self = Self(U256::ONE);
    pub const ZERO: Self = Self(U256::ZERO);

    pub const fn from_u128(value: u128) -> Self {
        let bytes = value.to_le_bytes();
        Self(U256::from_digits([
            u64::from_le_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            ]),
            u64::from_le_bytes([
                bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14],
                bytes[15],
            ]),
            0,
            0,
        ]))
    }

    pub const fn from_be_bytes(data: [u8; 32]) -> Self {
        Self(U256::from_digits([
            u64::from_le_bytes([
                data[31], data[30], data[29], data[28], data[27], data[26], data[25], data[24],
            ]),
            u64::from_le_bytes([
                data[23], data[22], data[21], data[20], data[19], data[18], data[17], data[16],
            ]),
            u64::from_le_bytes([
                data[15], data[14], data[13], data[12], data[11], data[10], data[9], data[8],
            ]),
            u64::from_le_bytes([
                data[7], data[6], data[5], data[4], data[3], data[2], data[1], data[0],
            ]),
        ]))
    }

    pub const fn from_le_bytes(data: [u8; 32]) -> Self {
        Self(U256::from_digits([
            u64::from_le_bytes([
                data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
            ]),
            u64::from_le_bytes([
                data[8], data[9], data[10], data[11], data[12], data[13], data[14], data[15],
            ]),
            u64::from_le_bytes([
                data[16], data[17], data[18], data[19], data[20], data[21], data[22], data[23],
            ]),
            u64::from_le_bytes([
                data[24], data[25], data[26], data[27], data[28], data[29], data[30], data[31],
            ]),
        ]))
    }

    pub const fn to_be_bytes(self) -> [u8; 32] {
        let words = self.0.digits();
        let words = [
            words[3].to_be_bytes(),
            words[2].to_be_bytes(),
            words[1].to_be_bytes(),
            words[0].to_be_bytes(),
        ];
        unsafe { mem::transmute(words) }
    }

    pub const fn to_le_bytes(self) -> [u8; 32] {
        let words = self.0.digits();
        let words = [
            words[0].to_le_bytes(),
            words[1].to_le_bytes(),
            words[2].to_le_bytes(),
            words[3].to_le_bytes(),
        ];
        unsafe { mem::transmute(words) }
    }

    pub const fn is_zero(self) -> bool {
        self.0.is_zero()
    }

    pub fn checked_add(self, other: Self) -> StdResult<Self> {
        self.0
            .checked_add(other.0)
            .map(Self)
            .ok_or_else(|| StdError::overflow_add(self, other))
    }

    pub fn checked_sub(self, other: Self) -> StdResult<Self> {
        self.0
            .checked_sub(other.0)
            .map(Self)
            .ok_or_else(|| StdError::overflow_sub(self, other))
    }

    pub fn checked_mul(self, other: Self) -> StdResult<Self> {
        self.0
            .checked_mul(other.0)
            .map(Self)
            .ok_or_else(|| StdError::overflow_mul(self, other))
    }

    pub fn checked_div(self, other: Self) -> StdResult<Self> {
        self.0
            .checked_div(other.0)
            .map(Self)
            .ok_or_else(|| StdError::division_by_zero(self))
    }

    pub fn checked_rem(self, other: Self) -> StdResult<Self> {
        self.0
            .checked_rem(other.0)
            .map(Self)
            .ok_or_else(|| StdError::remainder_by_zero(self))
    }

    pub fn checked_pow(self, exp: u32) -> StdResult<Self> {
        self.0
            .checked_pow(exp)
            .map(Self)
            .ok_or_else(|| StdError::overflow_pow(self, exp))
    }

    pub fn checked_shl(self, rhs: u32) -> StdResult<Self> {
        self.0
            .checked_shl(rhs)
            .map(Self)
            .ok_or_else(|| StdError::overflow_shl(self, rhs))
    }

    pub fn checked_shr(self, rhs: u32) -> StdResult<Self> {
        self.0
            .checked_shr(rhs)
            .map(Self)
            .ok_or_else(|| StdError::overflow_shr(self, rhs))
    }

    pub fn checked_multiply_ratio(self, nominator: Self, denominator: Self) -> StdResult<Self> {
        (Uint512::from(self) * Uint512::from(nominator) / Uint512::from(denominator)).try_into()
    }

    // note: unlike Uint64/128, there is no `checked_multiply_ratio` method for
    // Uint256, because to implement it we need a Uint512 type which we don't
    // have for now.

    /// Return the largest integer `n` such that `n * n <= self`.
    /// In other words, take the square root and _round down_.
    ///
    /// Copied from `uint` crate:
    /// <https://github.com/paritytech/parity-common/blob/uint-v0.9.5/uint/src/uint.rs#L963-L983>
    /// which utilizes the method described in:
    /// <https://en.wikipedia.org/wiki/Integer_square_root#Using_only_integer_division>
    pub fn integer_sqrt(self) -> Self {
        if self <= Self::ONE {
            return self;
        }

        let shift = (self.0.bits() + 1) / 2;
        let mut x_prev = Self::ONE << shift;
        loop {
            let x = (x_prev + self / x_prev) >> 1;
            if x >= x_prev {
                return x_prev;
            }
            x_prev = x;
        }
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

impl TryFrom<Uint256> for Uint128 {
    type Error = StdError;

    fn try_from(value: Uint256) -> StdResult<Self> {
        let bytes = value.to_le_bytes();
        let (lower, higher) = bytes.split_at(16);
        if higher != [0; 16] {
            return Err(StdError::overflow_conversion::<_, Uint128>(value));
        }
        Ok(Uint128::from_le_bytes(lower.try_into().unwrap()))
    }
}

impl FromStr for Uint256 {
    type Err = StdError;

    fn from_str(s: &str) -> StdResult<Self> {
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
        f.write_str("a string-encoded 256-bit unsigned integer")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        v.parse::<U256>().map(Uint256).map_err(E::custom)
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, proptest::prelude::*};

    proptest! {
        #[test]
        fn integer_sqrt(ref bytes in prop::array::uniform32(0..=u8::MAX)) {
            let a = Uint256::from_le_bytes(*bytes);
            let b = a.integer_sqrt();
            // b is sqrt(a) **floored**, so we need to make sure: b^2 <= a AND (b+1)^2 > a
            assert!(b * b <= a);
            assert!((b + Uint256::ONE) * (b + Uint256::ONE) > a);
        }
    }
}
