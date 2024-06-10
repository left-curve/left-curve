use {
    crate::{
        call_inner, forward_ref_binop_decimal, forward_ref_op_assign_decimal, generate_decimal,
        generate_decimal_per, generate_unchecked, impl_all_ops_and_assign, impl_assign_number,
        impl_number, Inner, IntPerDec, MultiplyRatio, NextNumber, Number, NumberConst, Rational,
        Sign, StdError, StdResult, Uint,
    },
    bnum::types::U256,
    borsh::{BorshDeserialize, BorshSerialize},
    forward_ref::{forward_ref_binop, forward_ref_op_assign},
    serde::{de, ser},
    std::{
        cmp::Ordering,
        fmt::{self, Display, Write},
        ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign},
        str::FromStr,
    },
};

// ------------------------------- generic type --------------------------------

#[derive(
    BorshSerialize, BorshDeserialize, Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord,
)]
pub struct Decimal<U, const S: usize>(pub(crate) Uint<U>);

impl<U, const S: usize> Inner for Decimal<U, S> {
    type U = U;
}

// --- Init ---
impl<U, const S: usize> Decimal<U, S> {
    /// Create a new [`Decimal`] _without_ adding decimal places.
    ///
    /// ```rust
    /// use {
    ///     grug_types::{Decimal128, Uint128},
    ///     std::str::FromStr,
    /// };
    ///
    /// let uint = Uint128::new(100);
    /// let decimal = Decimal128::raw(uint);
    /// assert_eq!(decimal, Decimal128::from_str("0.000000000000000100").unwrap());
    /// ```
    pub const fn raw(value: Uint<U>) -> Self {
        Self(value)
    }
}

impl<U, const S: usize> Decimal<U, S>
where
    Uint<U>: Number,
    U: NumberConst,
{
    fn f_pow(exp: u32) -> Uint<U> {
        Uint::TEN.checked_pow(exp).unwrap()
    }

    pub fn numerator(self) -> Uint<U> {
        self.0
    }

    pub fn decimal_fraction() -> Uint<U> {
        Self::f_pow(S as u32)
    }

    /// Create a new [`Decimal`] adding decimal places.
    ///
    /// ```rust
    /// use {
    ///     grug_types::{Decimal128, Uint128},
    ///     std::str::FromStr,
    /// };
    ///
    /// let uint = Uint128::new(100);
    /// let decimal = Decimal128::new(uint);
    /// assert_eq!(decimal, Decimal128::from_str("100.0").unwrap());
    /// ```
    pub fn new(value: impl Into<Uint<U>>) -> Self {
        Self(value.into() * Self::decimal_fraction())
    }

    pub(crate) fn from_decimal<OU, const OS: usize>(other: Decimal<OU, OS>) -> Self
    where
        Uint<U>: From<Uint<OU>>,
    {
        if OS > S {
            let adjusted_precision = Self::f_pow((OS - S) as u32);
            Self(Uint::<U>::from(other.0) / adjusted_precision)
        } else {
            let adjusted_precision = Self::f_pow((S - OS) as u32);
            Self(Uint::<U>::from(other.0) * adjusted_precision)
        }
    }

    pub(crate) fn try_from_decimal<OU, const OS: usize>(other: Decimal<OU, OS>) -> StdResult<Self>
    where
        Uint<U>: TryFrom<Uint<OU>, Error = StdError>,
    {
        if OS > S {
            let adjusted_precision = Self::f_pow((OS - S) as u32);
            Uint::<U>::try_from(other.0)
                .map(|val| val.checked_div(adjusted_precision))?
                .map(Self)
        } else {
            let adjusted_precision = Self::f_pow((S - OS) as u32);
            Uint::<U>::try_from(other.0)
                .map(|val| val.checked_mul(adjusted_precision))?
                .map(Self)
        }
    }
}

// --- Sign ---
impl<U, const S: usize> Sign for Decimal<U, S> {
    fn is_positive(&self) -> bool {
        true
    }
}

// --- Base impl ---
impl<U, const S: usize> Decimal<U, S>
where
    Uint<U>: Number,
    U: NumberConst + Clone + PartialEq + Copy + FromStr,
{
    generate_decimal_per!(percent, 2);

    generate_decimal_per!(permille, 4);

    generate_decimal_per!(bps, 6);

    generate_unchecked!(checked_ceil => ceil);

    generate_unchecked!(checked_from_atomics => from_atomics, args impl Into<Uint<U>>, u32);

    pub const fn zero() -> Self {
        Self(Uint::<U>::ZERO)
    }

    pub fn one() -> Self {
        Self(Self::decimal_fraction())
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
            floor.0.checked_add(Self::decimal_fraction()).map(Self)
        }
    }

    pub fn checked_from_atomics(
        atomics: impl Into<Uint<U>>,
        decimal_places: u32,
    ) -> StdResult<Self> {
        let atomics = atomics.into();

        let inner = match (decimal_places as usize).cmp(&S) {
            Ordering::Less => {
                // No overflow because decimal_places < S
                let digits = S as u32 - decimal_places;
                let factor = Uint::<U>::TEN.checked_pow(digits)?;
                atomics.checked_mul(factor)?
            },
            Ordering::Equal => atomics,
            Ordering::Greater => {
                // No overflow because decimal_places > S
                let digits = decimal_places - S as u32;
                if let Ok(factor) = Uint::<U>::TEN.checked_pow(digits) {
                    // Safe because factor cannot be zero
                    atomics.checked_div(factor).unwrap()
                } else {
                    // In this case `factor` exceeds the Uint<U> range.
                    // Any  Uint<U> `x` divided by `factor` with `factor > Uint::<U>::MAX` is 0.
                    // Try e.g. Python3: `(2**128-1) // 2**128`
                    Uint::<U>::ZERO
                }
            },
        };

        Ok(Self(inner))
    }
}

// --- Constants ---
impl<U, const S: usize> NumberConst for Decimal<U, S>
where
    U: NumberConst,
{
    const MAX: Self = Self::raw(Uint::new(U::MAX));
    const MIN: Self = Self::raw(Uint::new(U::MIN));
    const ONE: Self = Self::raw(Uint::new(U::ONE));
    const TEN: Self = Self::raw(Uint::new(U::TEN));
    const ZERO: Self = Self::raw(Uint::new(U::ZERO));
}

// --- Number ---
impl<U, const S: usize> Number for Decimal<U, S>
where
    Decimal<U, S>: ToString,
    Uint<U>: NextNumber + Number + PartialOrd,
    <Uint<U> as NextNumber>::Next: Number + ToString + Clone,
    U: NumberConst + Clone + PartialEq + Copy + FromStr,
{
    call_inner!(fn checked_add,    field 0, => Result<Self>);

    call_inner!(fn checked_sub,    field 0, => Result<Self>);

    call_inner!(fn wrapping_add,   field 0, => Self);

    call_inner!(fn wrapping_sub,   field 0, => Self);

    call_inner!(fn wrapping_mul,   field 0, => Self);

    call_inner!(fn wrapping_pow,   arg u32, => Self);

    call_inner!(fn saturating_add, field 0, => Self);

    call_inner!(fn saturating_sub, field 0, => Self);

    call_inner!(fn saturating_mul, field 0, => Self);

    call_inner!(fn saturating_pow, arg u32, => Self);

    call_inner!(fn abs,                     => Self);

    call_inner!(fn is_zero,                 => bool);

    fn checked_mul(self, other: Self) -> StdResult<Self> {
        let numerator = self.0.full_mul(other.numerator());
        let denominator = <Uint<U> as NextNumber>::Next::from(Self::decimal_fraction());
        let next_result = numerator.checked_div(denominator)?;
        Uint::<U>::try_from(next_result.clone())
            .map(Self)
            .map_err(|_| StdError::overflow_conversion::<_, Uint<U>>(next_result))
    }

    fn checked_div(self, other: Self) -> StdResult<Self> {
        Decimal::checked_from_ratio(self.numerator(), other.numerator())
    }

    fn checked_rem(self, other: Self) -> StdResult<Self> {
        self.0.checked_rem(other.0).map(Self)
    }

    fn checked_pow(mut self, mut exp: u32) -> StdResult<Self> {
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

    // TODO: Check if this is the best way to implement this
    fn checked_sqrt(self) -> StdResult<Self> {
        // With the current design, U should be only unsigned number.
        // Leave this safety check here for now.
        if self.0 < Uint::ZERO {
            return Err(StdError::negative_sqrt::<Self>(self));
        }
        let hundred = Uint::TEN.checked_mul(Uint::TEN)?;
        (0..=S as u32 / 2)
            .rev()
            .find_map(|i| -> Option<StdResult<Self>> {
                let inner_mul = match hundred.checked_pow(i) {
                    Ok(val) => val,
                    Err(err) => return Some(Err(err)),
                };
                self.0.checked_mul(inner_mul).ok().map(|inner| {
                    let outer_mul = hundred.checked_pow(S as u32 / 2 - i)?;
                    Ok(Self::raw(inner.checked_sqrt()?.checked_mul(outer_mul)?))
                })
            }).transpose()?
            // TODO: add a StdError variant to handle this?
            .ok_or(StdError::Generic("Sqrt failed".to_string()))
    }
}

// --- Checked from ratio (require Uint<U>: NextNumber) ---
impl<U, const S: usize> Decimal<U, S>
where
    Uint<U>: NextNumber + Number,
    <Uint<U> as NextNumber>::Next: Number + ToString + Clone,
    U: NumberConst + Clone + PartialEq + Copy + FromStr,
{
    generate_unchecked!(checked_from_ratio => from_ratio, args impl Into<Uint<U>>, impl Into<Uint<U>>);

    pub fn checked_from_ratio(
        numerator: impl Into<Uint<U>>,
        denominator: impl Into<Uint<U>>,
    ) -> StdResult<Self> {
        let numerator: Uint<U> = numerator.into();
        let denominator: Uint<U> = denominator.into();
        numerator
            .checked_multiply_ratio_floor(Self::decimal_fraction(), denominator)
            .map(Self)
    }
}

// --- Rational ---
impl<U, const S: usize> Rational<U> for Decimal<U, S>
where
    U: NumberConst + Number,
{
    fn numerator(self) -> Uint<U> {
        self.0
    }

    fn denominator() -> Uint<U> {
        Self::decimal_fraction()
    }
}

// --- IntperDecimal ---
impl<U, AsU, DR> IntPerDec<U, AsU, DR> for Uint<U>
where
    Uint<U>: MultiplyRatio,
    Uint<AsU>: Into<Uint<U>>,
    DR: Rational<AsU>,
    AsU: NumberConst + Number,
{
    fn checked_mul_dec_floor(self, rhs: DR) -> StdResult<Self> {
        self.checked_multiply_ratio_floor(rhs.numerator(), DR::denominator())
    }

    fn checked_mul_dec_ceil(self, rhs: DR) -> StdResult<Self> {
        self.checked_multiply_ratio_ceil(rhs.numerator(), DR::denominator())
    }

    fn checked_div_dec_floor(self, rhs: DR) -> StdResult<Self> {
        self.checked_multiply_ratio_floor(DR::denominator(), rhs.numerator())
    }

    fn checked_div_dec_ceil(self, rhs: DR) -> StdResult<Self> {
        self.checked_multiply_ratio_ceil(DR::denominator(), rhs.numerator())
    }
}

// --- Display ---
impl<U, const S: usize> Display for Decimal<U, S>
where
    Uint<U>: Number + Display + Copy,
    U: NumberConst + PartialEq + PartialOrd,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let decimals = Self::decimal_fraction();
        let whole = (self.0) / decimals;
        let fractional = (self.0).checked_rem(decimals).unwrap();

        if fractional.is_zero() {
            write!(f, "{whole}")?;
        } else {
            let fractional_string = format!("{:0>padding$}", fractional, padding = S);
            f.write_str(&whole.to_string())?;
            f.write_char('.')?;
            f.write_str(&fractional_string.trim_end_matches('0').replace('-', ""))?;
        }

        Ok(())
    }
}

// --- FromStr ---
impl<U, const S: usize> FromStr for Decimal<U, S>
where
    Uint<U>: Number + FromStr + Display,
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

        let whole = whole_part
            .parse::<Uint<U>>()
            .map_err(|_| StdError::generic_err("Error parsing whole"))?;
        let mut atomics = whole
            .checked_mul(decimal_fractional)
            .map_err(|_| StdError::generic_err("Value too big"))?;

        if let Some(fractional_part) = parts_iter.next() {
            let fractional = fractional_part
                .parse::<Uint<U>>()
                .map_err(|_| StdError::generic_err("Error parsing fractional"))?;
            let exp = (S.checked_sub(fractional_part.len())).ok_or_else(|| {
                StdError::generic_err(format!("Cannot parse more than {} fractional digits", S))
            })?;
            debug_assert!(exp <= S);

            let fractional_factor = Uint::TEN.checked_pow(exp as u32).unwrap();

            // This multiplication can't overflow because
            // fractional < 10^DECIMAL_PLACES && fractional_factor <= 10^DECIMAL_PLACES
            let fractional_part = fractional.checked_mul(fractional_factor).unwrap();

            // for negative numbers, we need to subtract the fractional part
            atomics = atomics
                .checked_add(fractional_part)
                .map_err(|_| StdError::generic_err("Value too big"))?;
        }

        if parts_iter.next().is_some() {
            return Err(StdError::generic_err("Unexpected number of dots"));
        }

        Ok(Decimal(atomics))
    }
}

// --- serde::Serialize ---
impl<U, const T: usize> ser::Serialize for Decimal<U, T>
where
    Self: Display,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

// --- serde::Deserialize ---
impl<'de, U, const S: usize> de::Deserialize<'de> for Decimal<U, S>
where
    U: Default + NumberConst + FromStr,
    <U as FromStr>::Err: Display,
    Uint<U>: Number + FromStr + Display,
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
    Uint<U>: Number + FromStr + Display,
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

impl_number!(impl Decimal with Add, add for Decimal<U, S> where sub fn checked_add);
impl_number!(impl Decimal with Sub, sub for Decimal<U, S> where sub fn checked_sub);
impl_number!(impl Decimal with Mul, mul for Decimal<U, S> where sub fn checked_mul);
impl_number!(impl Decimal with Div, div for Decimal<U, S> where sub fn checked_div);

impl_assign_number!(impl Decimal with AddAssign, add_assign for Decimal<U, S> where sub fn checked_add);
impl_assign_number!(impl Decimal with SubAssign, sub_assign for Decimal<U, S> where sub fn checked_sub);
impl_assign_number!(impl Decimal with MulAssign, mul_assign for Decimal<U, S> where sub fn checked_mul);
impl_assign_number!(impl Decimal with DivAssign, div_assign for Decimal<U, S> where sub fn checked_div);

forward_ref_binop_decimal!(impl Add, add for Decimal<U, S>, Decimal<U, S>);
forward_ref_binop_decimal!(impl Sub, sub for Decimal<U, S>, Decimal<U, S>);
forward_ref_binop_decimal!(impl Mul, mul for Decimal<U, S>, Decimal<U, S>);
forward_ref_binop_decimal!(impl Div, div for Decimal<U, S>, Decimal<U, S>);

forward_ref_op_assign_decimal!(impl AddAssign, add_assign for Decimal<U, S>, Decimal<U, S>);
forward_ref_op_assign_decimal!(impl SubAssign, sub_assign for Decimal<U, S>, Decimal<U, S>);
forward_ref_op_assign_decimal!(impl MulAssign, mul_assign for Decimal<U, S>, Decimal<U, S>);
forward_ref_op_assign_decimal!(impl DivAssign, div_assign for Decimal<U, S>, Decimal<U, S>);

// ------------------------------ concrete types -------------------------------

// Decimal128
generate_decimal!(
    name = Decimal128,
    inner_type = u128,
    decimal_places = 18,
    from_dec = [],
);

// Decimal256
generate_decimal!(
    name = Decimal256,
    inner_type = U256,
    decimal_places = 18,
    from_dec = [Decimal128],
);

// impl From<Decimal128> for Decimal256 {
//     fn from(value: Decimal128) -> Self {
//         todo!()
//     }
// }

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use bnum::{
        errors::TryFromIntError,
        types::{U256, U512},
    };

    use crate::{Decimal128, Decimal256, Number};

    #[test]
    fn t1() {
        assert_eq!(
            Decimal128::one() + Decimal128::one(),
            Decimal128::new(2_u128)
        );

        assert_eq!(
            Decimal128::new(10_u128)
                .checked_add(Decimal128::new(20_u128))
                .unwrap(),
            Decimal128::new(30_u128)
        );

        assert_eq!(
            Decimal128::new(3_u128)
                .checked_rem(Decimal128::new(2_u128))
                .unwrap(),
            Decimal128::from_str("1").unwrap()
        );

        assert_eq!(
            Decimal128::from_str("3.5")
                .unwrap()
                .checked_rem(Decimal128::new(2_u128))
                .unwrap(),
            Decimal128::from_str("1.5").unwrap()
        );

        assert_eq!(
            Decimal128::from_str("3.5")
                .unwrap()
                .checked_rem(Decimal128::from_str("2.7").unwrap())
                .unwrap(),
            Decimal128::from_str("0.8").unwrap()
        );
    }

    #[test]
    fn t2_conversion() {
        let u256 = U256::from(42_u64);
        let u512: U512 = u256.into();
        assert_eq!(u512, U512::from(42_u64));

        let u256: U256 = TryFrom::<U512>::try_from(u512).unwrap();
        assert_eq!(u256, U256::from(42_u64));

        let u256: Result<U256, TryFromIntError> = TryFrom::<U512>::try_from(U512::MAX);
        assert!(u256.is_err());

        let u256 = U256::MAX;
        let mut u512: U512 = u256.into();
        let _: U256 = TryFrom::<U512>::try_from(u512).unwrap();

        u512 += U512::ONE;

        let u256: Result<U256, TryFromIntError> = TryFrom::<U512>::try_from(U512::MAX);
        assert!(u256.is_err());
    }

    #[test]
    fn t3_conversion() {
        let foo = Decimal128::new(10_u128);
        assert_eq!(Decimal256::new(10_u128), Decimal256::from(foo));

        let foo = Decimal256::new(10_u128);
        assert_eq!(Decimal128::new(10_u128), Decimal128::try_from(foo).unwrap())
    }
}
