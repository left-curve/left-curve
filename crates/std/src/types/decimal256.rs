use {
    crate::{forward_ref_partial_eq, StdError, StdResult, Uint256, Uint512},
    forward_ref::{forward_ref_binop, forward_ref_op_assign},
    serde::{de, ser},
    std::{
        fmt,
        ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign},
        str::FromStr,
    },
};

/// 128-bit fixed point decimal number.
///
/// The number's low 18 digits are considered to be decimal places.
/// Meaning, for example, if the inner Uint128 is
///
/// ```plain
/// 123,456,789,000,000,000,000
/// ```
///
/// this represents the decimal number **123.456789**.
///
/// The maximum value that can be represented is
///
/// ```plain
/// Uint256::MAX / 10^18 ~= 115792089237316195423570985008687907853269984665640564039457.584007913129639935
/// ```
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Decimal256(Uint256);

forward_ref_partial_eq!(Decimal256, Decimal256);

impl Decimal256 {
    pub const DECIMAL_PLACES: u32 = 18;
    pub const DECIMAL_FRACTIONAL: Uint256 = Uint256::from_u128(1_000_000_000_000_000_000);

    pub const MAX:  Self = Self(Uint256::MAX);
    pub const MIN:  Self = Self(Uint256::MIN);
    pub const ZERO: Self = Self(Uint256::ZERO);
    pub const ONE:  Self = Self(Self::DECIMAL_FRACTIONAL);

    pub const fn is_zero(self) -> bool {
        self.0.is_zero()
    }

    pub fn checked_add(self, other: Self) -> StdResult<Self> {
        self.0
            .checked_add(other.0)
            .map(Self)
            .map_err(|_| StdError::overflow_add(self, other))
    }

    pub fn checked_sub(self, other: Self) -> StdResult<Self> {
        self.0
            .checked_sub(other.0)
            .map(Self)
            .map_err(|_| StdError::overflow_sub(self, other))
    }

    pub fn checked_mul(self, other: Self) -> StdResult<Self> {
        self.checked_multiply_ratio(other.0, Self::DECIMAL_FRACTIONAL)
    }

    pub fn checked_div(self, other: Self) -> StdResult<Self> {
        self.checked_multiply_ratio(Self::DECIMAL_FRACTIONAL, other.0)
    }

    pub fn checked_multiply_ratio(
        self,
        nominator:   Uint256,
        denominator: Uint256,
    ) -> StdResult<Self> {
        (Uint512::from(self.0) * Uint512::from(nominator) / Uint512::from(denominator))
            .try_into()
            .map(Self)
    }
}

impl Add for Decimal256 {
    type Output = Self;

    fn add(self, rhs: Decimal256) -> Self::Output {
        self.checked_add(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl Sub for Decimal256 {
    type Output = Self;

    fn sub(self, rhs: Decimal256) -> Self::Output {
        self.checked_sub(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl Mul for Decimal256 {
    type Output = Self;

    fn mul(self, rhs: Decimal256) -> Self::Output {
        self.checked_mul(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl Div for Decimal256 {
    type Output = Self;

    fn div(self, rhs: Decimal256) -> Self::Output {
        self.checked_div(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl AddAssign for Decimal256 {
    fn add_assign(&mut self, rhs: Decimal256) {
        *self = *self + rhs;
    }
}

impl SubAssign for Decimal256 {
    fn sub_assign(&mut self, rhs: Decimal256) {
        *self = *self - rhs;
    }
}

impl MulAssign for Decimal256 {
    fn mul_assign(&mut self, rhs: Decimal256) {
        *self = *self * rhs;
    }
}

impl DivAssign for Decimal256 {
    fn div_assign(&mut self, rhs: Decimal256) {
        *self = *self / rhs;
    }
}

forward_ref_binop!(impl Add, add for Decimal256, Decimal256);
forward_ref_binop!(impl Sub, sub for Decimal256, Decimal256);
forward_ref_binop!(impl Mul, mul for Decimal256, Decimal256);
forward_ref_binop!(impl Div, div for Decimal256, Decimal256);

forward_ref_op_assign!(impl AddAssign, add_assign for Decimal256, Decimal256);
forward_ref_op_assign!(impl SubAssign, sub_assign for Decimal256, Decimal256);
forward_ref_op_assign!(impl MulAssign, mul_assign for Decimal256, Decimal256);
forward_ref_op_assign!(impl DivAssign, div_assign for Decimal256, Decimal256);

// the range of Decimal256 is smaller than that of Uint256, so that cast may fail
impl TryFrom<Uint256> for Decimal256 {
    type Error = StdError;

    fn try_from(value: Uint256) -> StdResult<Self> {
        value.checked_mul(Self::DECIMAL_FRACTIONAL).map(Self)
    }
}

// allowed:
// - "1.23"
// - "1"
// - "000012"
// - "1.123000000"
// - "1."
// - ".23"
impl FromStr for Decimal256 {
    type Err = StdError;

    fn from_str(s: &str) -> StdResult<Self> {
        let mut parts = s.split('.');

        let whole_part = parts.next().unwrap(); // `.split` always return as least one part
        let whole = if whole_part.is_empty() {
            Uint256::ZERO
        } else {
            Uint256::from_str(whole_part)?.checked_mul(Self::DECIMAL_FRACTIONAL)?
        };

        let fraction = if let Some(fraction_part) = parts.next() {
            if fraction_part.is_empty() {
                Uint256::ZERO
            } else {
                let exp = Self::DECIMAL_PLACES.checked_sub(fraction_part.len() as u32)
                    .ok_or(StdError::deserialize::<Self>("too many decimal places"))?;
                let fractional_factor = Uint256::from_u128(10u128.pow(exp));
                Uint256::from_str(fraction_part)?.checked_mul(fractional_factor)?
            }
        } else {
            Uint256::ZERO
        };

        if parts.next().is_some() {
            return Err(StdError::deserialize::<Self>("unexpected number of dots"));
        }

        whole.checked_add(fraction).map(Self)
    }
}

impl From<Decimal256> for String {
    fn from(value: Decimal256) -> Self {
        value.to_string()
    }
}

impl fmt::Display for Decimal256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let whole = self.0 / Self::DECIMAL_FRACTIONAL;
        let fractional = self.0 % Self::DECIMAL_FRACTIONAL;

        if fractional.is_zero() {
            write!(f, "{whole}")
        } else {
            let s = format!("{whole}.{fractional:0>padding$}", padding = Self::DECIMAL_PLACES as usize);
            f.write_str(s.trim_end_matches('0'))
        }
    }
}

impl fmt::Debug for Decimal256 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Decimal({self})")
    }
}

impl ser::Serialize for Decimal256 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> de::Deserialize<'de> for Decimal256 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_str(Decimal256Visitor)
    }
}

struct Decimal256Visitor;

impl<'de> de::Visitor<'de> for Decimal256Visitor {
    type Value = Decimal256;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a string-encoded 256-bit fixed point unsigned decimal")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Decimal256::from_str(v).map_err(E::custom)
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{from_json, to_json},
        test_case::test_case,
    };

    #[test]
    fn multiplication() {
        let lhs = Decimal256::from_str("74567.67567654").unwrap();
        let rhs = Decimal256::from_str("42143.34452434").unwrap();
        assert_eq!(lhs * rhs, Decimal256::from_str("3142531246.4156730139969836").unwrap());
    }

    #[test]
    fn division() {
        let lhs = Decimal256::from_str("74567.67567654").unwrap();
        let rhs = Decimal256::from_str("42143.34452434").unwrap();
        assert_eq!(lhs / rhs, Decimal256::from_str("1.76938201081").unwrap());
    }

    #[test_case(
        Uint256::from_u128(1230000000000000000),
        b"\"1.23\"";
        "both whole and fraction"
    )]
    #[test_case(
        Uint256::from_u128(1000000000000000000),
        b"\"1\"";
        "only whole"
    )]
    #[test_case(
        Uint256::from_u128(230000000000000000),
        b"\"0.23\"";
        "only fraction"
    )]
    #[test_case(
        Uint256::ZERO,
        b"\"0\"";
        "zero"
    )]
    #[test_case(
        Uint256::MAX,
        b"\"115792089237316195423570985008687907853269984665640564039457.584007913129639935\"";
        "max"
    )]
    fn serialization(inner: Uint256, output: &[u8]) {
        let decimal = Decimal256(inner);
        assert_eq!(to_json(&decimal).unwrap().as_ref(), output);
    }

    #[test_case(
        Some(Uint256::from_u128(1230000000000000000)),
        b"\"1.23\"";
        "both whole and fraction"
    )]
    #[test_case(
        Some(Uint256::from_u128(1000000000000000000)),
        b"\"1\"";
        "only whole no decimal point"
    )]
    #[test_case(
        Some(Uint256::from_u128(1000000000000000000)),
        b"\"1.\"";
        "only whole with decimal point"
    )]
    #[test_case(
        Some(Uint256::from_u128(230000000000000000)),
        b"\".23\"";
        "only fraction with decimal point"
    )]
    #[test_case(
        Some(Uint256::from_u128(1230000000000000000)),
        b"\"00001.23\"";
        "leading zeros"
    )]
    #[test_case(
        Some(Uint256::from_u128(1230000000000000000)),
        b"\"1.230000\"";
        "trailing zeros"
    )]
    #[test_case(
        Some(Uint256::ZERO),
        b"\"0\"";
        "zero"
    )]
    #[test_case(
        Some(Uint256::MAX),
        b"\"115792089237316195423570985008687907853269984665640564039457.584007913129639935\"";
        "max"
    )]
    #[test_case(
        None,
        b"\"1.2.3\"";
        "incorrect number of dots"
    )]
    #[test_case(
        None,
        b"\"larry.123\"";
        "invalid whole part"
    )]
    #[test_case(
        None,
        b"\"123.larry\"";
        "invalid fraction part"
    )]
    fn deserialization(inner: Option<Uint256>, input: &[u8]) {
        let result = from_json::<Decimal256>(input);
        if let Some(inner) = inner {
            let decimal = Decimal256(inner);
            assert_eq!(result.unwrap(), decimal);
        } else {
            assert!(result.is_err());
        }
    }
}
