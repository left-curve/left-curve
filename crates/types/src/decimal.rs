use {
    crate::{
        forward_ref_binop_decimal, forward_ref_op_assign_decimal, generate_decimal,
        generate_decimal_per, generate_unchecked, impl_all_ops_and_assign, impl_assign,
        impl_base_ops, impl_signed_ops, CheckedOps, DecimalRef, Inner, Int, IntPerDec, NextNumber,
        NumberConst, Sqrt, StdError, StdResult,
    },
    bnum::types::{I256, U256},
    borsh::{BorshDeserialize, BorshSerialize},
    forward_ref::{forward_ref_binop, forward_ref_op_assign},
    serde::{de, ser},
    std::{
        cmp::Ordering,
        fmt::{self, Display, Write},
        ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Not, Sub, SubAssign},
        str::FromStr,
    },
};

// ------------------------------- generic type --------------------------------

#[derive(
    BorshSerialize, BorshDeserialize, Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord,
)]
pub struct Decimal<U, const S: usize>(pub(crate) Int<U>);

impl<U, const S: usize> Inner for Decimal<U, S> {
    type U = U;
}

// --- Init ---
impl<U, const S: usize> Decimal<U, S>
where
    Int<U>: CheckedOps,
    U: NumberConst,
{
    fn f_pow(exp: u32) -> Int<U> {
        Int::TEN.checked_pow(exp).unwrap()
    }

    pub fn numerator(self) -> Int<U> {
        self.0
    }

    pub fn decimal_fraction() -> Int<U> {
        Self::f_pow(S as u32)
    }

    pub fn new(value: impl Into<Int<U>>) -> Self {
        Self(value.into() * Self::decimal_fraction())
    }

    pub const fn raw(value: Int<U>) -> Self {
        Self(value)
    }
}

// --- Base impl ---
impl<U, const S: usize> Decimal<U, S>
where
    Int<U>: CheckedOps,
    U: NumberConst + Clone + PartialEq + Copy + FromStr,
{
    generate_decimal_per!(percent, 2);
    generate_decimal_per!(permille, 4);
    generate_decimal_per!(bps, 6);
    generate_unchecked!(checked_ceil => ceil);
    generate_unchecked!(checked_from_atomics => from_atomics, args impl Into<Int<U>>, u32);

    pub const fn zero() -> Self {
        Self(Int::<U>::ZERO)
    }

    pub fn one() -> Self {
        Self(Self::decimal_fraction())
    }

    pub fn is_zero(self) -> bool {
        self.0.is_zero()
    }

    pub fn checked_add(self, rhs: Self) -> StdResult<Self> {
        self.0.checked_add(rhs.0).map(Self)
    }

    pub fn checked_sub(self, rhs: Self) -> StdResult<Self> {
        self.0.checked_sub(rhs.0).map(Self)
    }

    pub fn floor(self) -> Self {
        let decimal_fraction = Self::decimal_fraction();
        Self((self.0 / decimal_fraction) * decimal_fraction)
    }

    pub fn checked_ceil(self) -> StdResult<Self> {
        let floor = self.floor();
        if floor == self {
            Ok(floor)
        } else {
            floor.checked_add(Self::one())
        }
    }

    pub fn checked_from_atomics(
        atomics: impl Into<Int<U>>,
        decimal_places: u32,
    ) -> StdResult<Self> {
        let atomics = atomics.into();

        let inner = match (decimal_places as usize).cmp(&S) {
            Ordering::Less => {
                // No overflow because decimal_places < S
                let digits = S as u32 - decimal_places;
                let factor = Int::<U>::TEN.checked_pow(digits)?;
                atomics.checked_mul(factor)?
            },
            Ordering::Equal => atomics,
            Ordering::Greater => {
                // No overflow because decimal_places > S
                let digits = decimal_places - S as u32;
                if let Ok(factor) = Int::<U>::TEN.checked_pow(digits) {
                    // Safe because factor cannot be zero
                    atomics.checked_div(factor).unwrap()
                } else {
                    // In this case `factor` exceeds the Int<U> range.
                    // Any  Int<U> `x` divided by `factor` with `factor > Int::<U>::MAX` is 0.
                    // Try e.g. Python3: `(2**128-1) // 2**128`
                    Int::<U>::ZERO
                }
            },
        };

        Ok(Self(inner))
    }
}

// --- Mul / Div (Require Int<U>: NextNumber) ---
impl<U, const S: usize> Decimal<U, S>
where
    Int<U>: NextNumber + CheckedOps,
    <Int<U> as NextNumber>::Next: From<Int<U>> + TryInto<Int<U>> + CheckedOps + ToString + Clone,
    U: NumberConst + Clone + PartialEq + Copy + FromStr,
{
    generate_unchecked!(checked_pow => pow, arg u32);
    generate_unchecked!(checked_from_ratio => from_ratio, args impl Into<Int<U>>, impl Into<Int<U>>);

    pub fn checked_from_ratio(
        numerator: impl Into<Int<U>>,
        denominator: impl Into<Int<U>>,
    ) -> StdResult<Self> {
        let numerator: Int<U> = numerator.into();
        let denominator: Int<U> = denominator.into();
        numerator
            .checked_multiply_ratio_floor(Self::decimal_fraction(), denominator)
            .map(Self)
    }

    pub fn checked_mul(self, rhs: Self) -> StdResult<Self> {
        let numerator = self.0.full_mul(rhs.numerator());
        let denominator = <Int<U> as NextNumber>::Next::from(Self::decimal_fraction());
        let next_result = numerator.checked_div(denominator)?;
        TryInto::<Int<U>>::try_into(next_result.clone())
            .map(Self)
            .map_err(|_| StdError::overflow_conversion::<_, Int<U>>(next_result))
    }

    pub fn checked_div(self, rhs: Self) -> StdResult<Self> {
        Decimal::checked_from_ratio(self.numerator(), rhs.numerator())
    }

    pub fn checked_pow(mut self, mut exp: u32) -> StdResult<Self> {
        if exp == 0 {
            return Ok(Decimal::zero());
        }

        let mut y = Decimal::one();

        while exp > 1 {
            if exp % 2 == 0 {
                self = self.checked_mul(self)?;
                exp /= 2;
            } else {
                y = self.checked_mul(y)?;
                self = self.checked_mul(self)?;
                exp = (exp - 1) / 2;
            }
        }

        Ok(self * y)
    }
}

// --- Sqrt ---
impl<U, const S: usize> Sqrt for Decimal<U, S>
where
    Decimal<U, S>: ToString,
    Int<U>: CheckedOps + NumberConst + Sqrt + Copy + PartialOrd + PartialEq,
    U: NumberConst,
{
    fn checked_sqrt(self) -> StdResult<Self> {
        if self.0 < Int::ZERO {
            return Err(StdError::negative_sqrt::<Self>(self));
        }
        let hundred = Int::TEN.checked_mul(Int::TEN)?;
        (0..=S as u32 / 2)
            .rev()
            .find_map(|i| {
                let inner_mul = hundred.checked_pow(i).unwrap();
                self.0.checked_mul(inner_mul).ok().map(|inner| {
                    let outer_mul = hundred.checked_pow(S as u32 / 2 - i).unwrap();
                    Self::raw(inner.sqrt().checked_mul(outer_mul).unwrap())
                })
            })
            .ok_or(StdError::Generic("Sqrt failed".to_string())) // TODO: add a StdError variant to handle this?
    }
}

// --- DecimalRef ---
impl<U, const S: usize> DecimalRef<U> for Decimal<U, S>
where
    U: NumberConst + CheckedOps,
{
    fn numerator(self) -> Int<U> {
        self.0
    }

    fn denominator() -> Int<U> {
        Self::decimal_fraction()
    }
}

// --- IntperDecimal ---

impl<U, AsU, DR> IntPerDec<U, AsU, DR> for Int<U>
where
    Int<AsU>: Into<Int<U>>,
    DR: DecimalRef<AsU>,
    AsU: NumberConst + CheckedOps,
    U: NumberConst + CheckedOps + PartialEq,
    Int<U>: NextNumber + CheckedOps + Copy,
    <Int<U> as NextNumber>::Next: From<Int<U>> + TryInto<Int<U>> + CheckedOps + ToString + Clone,
{
    fn checked_mul_dec_floor(self, rhs: DR) -> StdResult<Self> {
        self.checked_multiply_ratio_floor(rhs.numerator(), DR::denominator())
    }

    fn mul_dec_floor(self, rhs: DR) -> Self {
        self.checked_mul_dec_floor(rhs).unwrap()
    }

    fn checked_mul_dec_ceil(self, rhs: DR) -> StdResult<Self> {
        self.checked_multiply_ratio_ceil(rhs.numerator(), DR::denominator())
    }

    fn mul_dec_ceil(self, rhs: DR) -> Self {
        self.checked_mul_dec_ceil(rhs).unwrap()
    }

    fn checked_div_dec_floor(self, rhs: DR) -> StdResult<Self> {
        self.checked_multiply_ratio_floor(DR::denominator(), rhs.numerator())
    }

    fn div_dec_floor(self, rhs: DR) -> Self {
        self.checked_div_dec_floor(rhs).unwrap()
    }

    fn checked_div_dec_ceil(self, rhs: DR) -> StdResult<Self> {
        self.checked_multiply_ratio_ceil(DR::denominator(), rhs.numerator())
    }

    fn div_dec_ceil(self, rhs: DR) -> Self {
        self.checked_div_dec_ceil(rhs).unwrap()
    }
}

impl_signed_ops!(impl Not for Decimal);
impl_signed_ops!(impl Neg for Decimal);

impl_base_ops!(impl Decimal with Add, add for Decimal<U, S> where sub fn checked_add);
impl_base_ops!(impl Decimal with Sub, sub for Decimal<U, S> where sub fn checked_sub);
impl_base_ops!(impl Decimal with Mul, mul for Decimal<U, S> where sub fn checked_mul);
impl_base_ops!(impl Decimal with Div, div for Decimal<U, S> where sub fn checked_div);

impl_assign!(impl Decimal with AddAssign, add_assign for Decimal<U, S> where sub fn checked_add);
impl_assign!(impl Decimal with SubAssign, sub_assign for Decimal<U, S> where sub fn checked_sub);
impl_assign!(impl Decimal with MulAssign, mul_assign for Decimal<U, S> where sub fn checked_mul);
impl_assign!(impl Decimal with DivAssign, div_assign for Decimal<U, S> where sub fn checked_div);

forward_ref_binop_decimal!(impl Add, add for Decimal<U, S>, Decimal<U, S>);
forward_ref_binop_decimal!(impl Sub, sub for Decimal<U, S>, Decimal<U, S>);
forward_ref_binop_decimal!(impl Mul, mul for Decimal<U, S>, Decimal<U, S>);
forward_ref_binop_decimal!(impl Div, div for Decimal<U, S>, Decimal<U, S>);

forward_ref_op_assign_decimal!(impl AddAssign, add_assign for Decimal<U, S>, Decimal<U, S>);
forward_ref_op_assign_decimal!(impl SubAssign, sub_assign for Decimal<U, S>, Decimal<U, S>);
forward_ref_op_assign_decimal!(impl MulAssign, mul_assign for Decimal<U, S>, Decimal<U, S>);
forward_ref_op_assign_decimal!(impl DivAssign, div_assign for Decimal<U, S>, Decimal<U, S>);

impl<U, const S: usize> Display for Decimal<U, S>
where
    Int<U>: CheckedOps + Display + Copy,
    U: NumberConst + PartialEq + PartialOrd,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let decimals = Self::decimal_fraction();
        let whole = (self.0) / decimals;
        let fractional = (self.0).checked_rem(decimals).unwrap();

        if whole < Int::ZERO || fractional < Int::ZERO {
            write!(f, "-")?;
        }

        if fractional.is_zero() {
            write!(f, "{whole}")?;
        } else {
            let fractional_string = format!("{:0>padding$}", fractional.abs(), padding = S);
            f.write_str(&whole.abs().to_string())?;
            f.write_char('.')?;
            f.write_str(&fractional_string.trim_end_matches('0').replace("-", ""))?;
        }

        Ok(())
    }
}

impl<U, const S: usize> FromStr for Decimal<U, S>
where
    Int<U>: CheckedOps + FromStr + Display,
    U: NumberConst,
{
    type Err = StdError;

    /// Converts the decimal string to a Decimal
    /// Possible inputs: "1.23", "1", "000012", "1.123000000"
    /// Disallowed: "", ".23"
    ///
    /// This never performs any kind of rounding.
    /// More than DECIMAL_PLACES fractional digits, even zeros, result in an error.
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut parts_iter = input.split('.');

        let decimal_fractional = Self::decimal_fraction();

        let whole_part = parts_iter.next().unwrap(); // split always returns at least one element
        let is_neg = whole_part.starts_with('-');

        let whole = whole_part
            .parse::<Int<U>>()
            .map_err(|_| StdError::generic_err("Error parsing whole"))?;
        let mut atomics = whole
            .checked_mul(decimal_fractional)
            .map_err(|_| StdError::generic_err("Value too big"))?;

        if let Some(fractional_part) = parts_iter.next() {
            let fractional = fractional_part
                .parse::<Int<U>>()
                .map_err(|_| StdError::generic_err("Error parsing fractional"))?;
            let exp = (S.checked_sub(fractional_part.len())).ok_or_else(|| {
                StdError::generic_err(format!("Cannot parse more than {} fractional digits", S))
            })?;
            debug_assert!(exp <= S);

            let fractional_factor = Int::TEN.checked_pow(exp as u32).unwrap();

            // This multiplication can't overflow because
            // fractional < 10^DECIMAL_PLACES && fractional_factor <= 10^DECIMAL_PLACES
            let fractional_part = Int::from(fractional)
                .checked_mul(fractional_factor)
                .unwrap();

            // for negative numbers, we need to subtract the fractional part
            atomics = if is_neg {
                atomics.checked_sub(fractional_part)
            } else {
                atomics.checked_add(fractional_part)
            }
            .map_err(|_| StdError::generic_err("Value too big"))?;
        }

        if parts_iter.next().is_some() {
            return Err(StdError::generic_err("Unexpected number of dots"));
        }

        Ok(Decimal(atomics))
    }
}

impl<U, const T: usize> ser::Serialize for Decimal<U, T>
where
    U: Display,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de, U, const S: usize> de::Deserialize<'de> for Decimal<U, S>
where
    U: Default + NumberConst + FromStr,
    <U as FromStr>::Err: Display,
    Int<U>: CheckedOps + FromStr + Display,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_str(DecimalVisitor::<U, S>::default())
    }
}

#[derive(Default)]
struct DecimalVisitor<U, const S: usize> {
    _marker: std::marker::PhantomData<U>,
}

impl<'de, U, const S: usize> de::Visitor<'de> for DecimalVisitor<U, S>
where
    U: NumberConst,
    Int<U>: CheckedOps + FromStr + Display,
{
    type Value = Decimal<U, S>;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // TODO: Change this message in base at the type of U
        f.write_str("string-encoded decimal")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match Self::Value::from_str(v) {
            Ok(d) => Ok(d),
            Err(e) => Err(E::custom(format_args!("Error parsing decimal '{v}': {e}"))),
        }
    }
}

// ------------------------------ concrete types -------------------------------

// Decimal128
generate_decimal!(
    name = Decimal128,
    inner_type = u128,
    decimal_places = 18,
    from_dec = []
);

// Decimal256
generate_decimal!(
    name = Decimal256,
    inner_type = U256,
    decimal_places = 18,
    from_dec = [Decimal128]
);

// SignedDecimal128
generate_decimal!(
    name = SignedDecimal128,
    inner_type = i128,
    decimal_places = 18,
    from_dec = [],
    try_from_dec = [Decimal128]
);

// SignedDecimal256
generate_decimal!(
    name = SignedDecimal256,
    inner_type = I256,
    decimal_places = 18,
    from_dec = [SignedDecimal128, Decimal128],
    try_from_dec = [Decimal256]
);

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{Int256, Uint128, Uint256},
    };

    #[test]
    fn t1() {
        let val = SignedDecimal128::from_str("-2").unwrap();
        assert_eq!(val * val, SignedDecimal128::from_str("4").unwrap());

        let value = Uint128::new(100);
        let dec = Decimal128::from_str("2.5").unwrap();
        let result = value.checked_mul_dec_floor(dec).unwrap();
        assert_eq!(result, Uint128::new(250));

        let value: Uint256 = 100_u128.into();
        let dec = Decimal128::from_str("2.5").unwrap();
        let result = value.checked_mul_dec_floor(dec).unwrap();
        assert_eq!(result, Uint256::new(250_u128.into()));

        let value: Int256 = (-10_i128).into();
        let dec = Decimal128::from_str("2.5").unwrap();
        assert_eq!(value.mul_dec_floor(dec), Int256::new_from(-25));

        let foo = Decimal128::new(10_u64);
        let bar = Decimal256::new(10_u128);
        assert_eq!(bar + foo, Decimal256::new(20_u128));
        assert_eq!(foo + bar, Decimal256::new(20_u128));

        let foo = SignedDecimal128::from_str("-10").unwrap();
        assert_eq!(-foo, SignedDecimal128::from_str("10").unwrap());

        let foo: Decimal128 = Decimal128::new(10_u64);
        assert_eq!(
            TryInto::<Decimal256>::try_into(foo).unwrap(),
            Decimal256::new(10_u128)
        );

        let foo: Decimal128 = Decimal128::new(10_u64);
        assert_eq!(
            TryInto::<SignedDecimal128>::try_into(foo).unwrap(),
            SignedDecimal128::new(10_u64)
        );

        let foo: Decimal128 = Decimal128::raw(u128::MAX.into());
        TryInto::<SignedDecimal128>::try_into(foo).unwrap_err();
    }
}
