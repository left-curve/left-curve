use {
    crate::{forward_ref_partial_eq, StdError, StdResult, Uint128, Uint256},
    borsh::{BorshDeserialize, BorshSerialize},
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
/// Uint128::MAX / 10^18 = 340282366920938463463.374607431768211455
/// ```
#[derive(
    BorshSerialize, BorshDeserialize, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord,
)]
pub struct Decimal(Uint128);

forward_ref_partial_eq!(Decimal, Decimal);

impl Decimal {
    pub const DECIMAL_FRACTIONAL: Uint128 = Uint128::new(1_000_000_000_000_000_000);
    pub const DECIMAL_PLACES: u32 = 18;
    pub const MAX: Self = Self(Uint128::MAX);
    pub const MIN: Self = Self(Uint128::MIN);
    pub const ONE: Self = Self(Self::DECIMAL_FRACTIONAL);
    pub const ZERO: Self = Self(Uint128::ZERO);

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
        nominator: Uint128,
        denominator: Uint128,
    ) -> StdResult<Self> {
        (Uint256::from(self.0) * Uint256::from(nominator) / Uint256::from(denominator))
            .try_into()
            .map(Self)
    }
}

impl Add for Decimal {
    type Output = Self;

    fn add(self, rhs: Decimal) -> Self::Output {
        self.checked_add(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl Sub for Decimal {
    type Output = Self;

    fn sub(self, rhs: Decimal) -> Self::Output {
        self.checked_sub(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl Mul for Decimal {
    type Output = Self;

    fn mul(self, rhs: Decimal) -> Self::Output {
        self.checked_mul(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl Div for Decimal {
    type Output = Self;

    fn div(self, rhs: Decimal) -> Self::Output {
        self.checked_div(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl AddAssign for Decimal {
    fn add_assign(&mut self, rhs: Decimal) {
        *self = *self + rhs;
    }
}

impl SubAssign for Decimal {
    fn sub_assign(&mut self, rhs: Decimal) {
        *self = *self - rhs;
    }
}

impl MulAssign for Decimal {
    fn mul_assign(&mut self, rhs: Decimal) {
        *self = *self * rhs;
    }
}

impl DivAssign for Decimal {
    fn div_assign(&mut self, rhs: Decimal) {
        *self = *self / rhs;
    }
}

forward_ref_binop!(impl Add, add for Decimal, Decimal);
forward_ref_binop!(impl Sub, sub for Decimal, Decimal);
forward_ref_binop!(impl Mul, mul for Decimal, Decimal);
forward_ref_binop!(impl Div, div for Decimal, Decimal);

forward_ref_op_assign!(impl AddAssign, add_assign for Decimal, Decimal);
forward_ref_op_assign!(impl SubAssign, sub_assign for Decimal, Decimal);
forward_ref_op_assign!(impl MulAssign, mul_assign for Decimal, Decimal);
forward_ref_op_assign!(impl DivAssign, div_assign for Decimal, Decimal);

// the range of Decimal is smaller than that of Uint128, so that cast may fail
impl TryFrom<Uint128> for Decimal {
    type Error = StdError;

    fn try_from(value: Uint128) -> StdResult<Self> {
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
impl FromStr for Decimal {
    type Err = StdError;

    fn from_str(s: &str) -> StdResult<Self> {
        let mut parts = s.split('.');

        let whole_part = parts.next().unwrap(); // `.split` always return as least one part
        let whole = if whole_part.is_empty() {
            Uint128::ZERO
        } else {
            Uint128::from_str(whole_part)?.checked_mul(Self::DECIMAL_FRACTIONAL)?
        };

        let fraction = if let Some(fraction_part) = parts.next() {
            if fraction_part.is_empty() {
                Uint128::ZERO
            } else {
                let exp = Self::DECIMAL_PLACES
                    .checked_sub(fraction_part.len() as u32)
                    .ok_or(StdError::deserialize::<Self>("too many decimal places"))?;
                let fractional_factor = Uint128::new(10u128.pow(exp));
                Uint128::from_str(fraction_part)?.checked_mul(fractional_factor)?
            }
        } else {
            Uint128::ZERO
        };

        if parts.next().is_some() {
            return Err(StdError::deserialize::<Self>("unexpected number of dots"));
        }

        whole.checked_add(fraction).map(Self)
    }
}

impl From<Decimal> for String {
    fn from(value: Decimal) -> Self {
        value.to_string()
    }
}

impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let whole = self.0 / Self::DECIMAL_FRACTIONAL;
        let fractional = self.0 % Self::DECIMAL_FRACTIONAL;

        if fractional.is_zero() {
            write!(f, "{whole}")
        } else {
            let s = format!(
                "{whole}.{fractional:0>padding$}",
                padding = Self::DECIMAL_PLACES as usize
            );
            f.write_str(s.trim_end_matches('0'))
        }
    }
}

impl fmt::Debug for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Decimal({self})")
    }
}

impl ser::Serialize for Decimal {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> de::Deserialize<'de> for Decimal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_str(DecimalVisitor)
    }
}

struct DecimalVisitor;

impl<'de> de::Visitor<'de> for DecimalVisitor {
    type Value = Decimal;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a string-encoded 128-bit fixed point unsigned decimal")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Decimal::from_str(v).map_err(E::custom)
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{from_json_value, to_json_value, Json},
        serde_json::json,
        test_case::test_case,
    };

    #[test]
    fn multiplication() {
        let lhs = Decimal::from_str("74567.67567654").unwrap();
        let rhs = Decimal::from_str("42143.34452434").unwrap();
        assert_eq!(
            lhs * rhs,
            Decimal::from_str("3142531246.4156730139969836").unwrap()
        );
    }

    #[test]
    fn division() {
        let lhs = Decimal::from_str("74567.67567654").unwrap();
        let rhs = Decimal::from_str("42143.34452434").unwrap();
        // note: keep 18 decimal places
        assert_eq!(
            lhs / rhs,
            Decimal::from_str("1.769382010805365527").unwrap()
        );
    }

    #[test_case(
        1230000000000000000,
        json!("1.23");
        "both whole and fraction"
    )]
    #[test_case(
        1000000000000000000,
        json!("1");
        "only whole"
    )]
    #[test_case(
        230000000000000000,
        json!("0.23");
        "only fraction"
    )]
    #[test_case(
        0,
        json!("0");
        "zero"
    )]
    #[test_case(
        u128::MAX,
        json!("340282366920938463463.374607431768211455");
        "max"
    )]
    fn serialization(inner: u128, output: Json) {
        let decimal = Decimal(inner.into());
        assert_eq!(to_json_value(&decimal).unwrap(), output);
    }

    #[test_case(
        Some(1230000000000000000),
        json!("1.23");
        "both whole and fraction"
    )]
    #[test_case(
        Some(1000000000000000000),
        json!("1");
        "only whole no decimal point"
    )]
    #[test_case(
        Some(1000000000000000000),
        json!("1.");
        "only whole with decimal point"
    )]
    #[test_case(
        Some(230000000000000000),
        json!(".23");
        "only fraction with decimal point"
    )]
    #[test_case(
        Some(1230000000000000000),
        json!("00001.23");
        "leading zeros"
    )]
    #[test_case(
        Some(1230000000000000000),
        json!("1.230000");
        "trailing zeros"
    )]
    #[test_case(
        Some(0),
        json!("0");
        "zero"
    )]
    #[test_case(
        Some(u128::MAX),
        json!("340282366920938463463.374607431768211455");
        "max"
    )]
    #[test_case(
        None,
        json!("1.2.3");
        "incorrect number of dots"
    )]
    #[test_case(
        None,
        json!("larry.123");
        "invalid whole part"
    )]
    #[test_case(
        None,
        json!("123.larry");
        "invalid fraction part"
    )]
    fn deserialization(inner: Option<u128>, input: Json) {
        let result = from_json_value::<Decimal>(input);
        if let Some(inner) = inner {
            let decimal = Decimal(inner.into());
            assert_eq!(result.unwrap(), decimal);
        } else {
            assert!(result.is_err());
        }
    }
}
