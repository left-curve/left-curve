use std::{
    fmt::{Display, Write},
    str::FromStr,
};

use crate::{
    generate_decimal_per, generate_unchecked, impl_base_ops, CheckedOps, NumberConst, NextNumber,
    Sqrt, StdError, StdResult, Uint,
};

#[derive(Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Decimal<U, const S: usize>(Uint<U>);

impl<U, const S: usize> Decimal<U, S>
where
    Uint<U>: CheckedOps,
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

    pub fn new(value: impl Into<Uint<U>>) -> Self {
        Self(value.into() * Self::decimal_fraction())
    }

    pub const fn raw(value: Uint<U>) -> Self {
        Self(value)
    }
}

impl<U, const S: usize> Decimal<U, S>
where
    Uint<U>: CheckedOps,
    U: NumberConst + Clone + PartialEq + Copy + FromStr,
{
    pub const fn zero() -> Self {
        Self(Uint::<U>::ZERO)
    }

    pub fn one() -> Self {
        Self(Self::decimal_fraction())
    }

    pub fn is_zero(self) -> bool {
        self.0.is_zero()
    }

    generate_decimal_per!(percent, 2);
    generate_decimal_per!(permille, 4);
    generate_decimal_per!(bps, 6);

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

    generate_unchecked!(checked_ceil => ceil);

    pub fn checked_from_atomics(
        atomics: impl Into<Uint<U>>,
        decimal_places: u32,
    ) -> StdResult<Self> {
        let atomics = atomics.into();

        Ok(match (decimal_places as usize).cmp(&S) {
            std::cmp::Ordering::Less => {
                let digits = S as u32 - decimal_places; // No overflow because decimal_places < S
                let factor = Uint::<U>::TEN.checked_pow(digits)?;
                Self(atomics.checked_mul(factor)?)
            },
            std::cmp::Ordering::Equal => Self(atomics),
            std::cmp::Ordering::Greater => {
                let digits = decimal_places - S as u32; // No overflow because decimal_places > S
                if let Ok(factor) = Uint::<U>::TEN.checked_pow(digits) {
                    Self(atomics.checked_div(factor).unwrap()) // Safe because factor cannot be zero
                } else {
                    // In this case `factor` exceeds the Uint<U> range.
                    // Any  Uint<U> `x` divided by `factor` with `factor > Uint::<U>::MAX` is 0.
                    // Try e.g. Python3: `(2**128-1) // 2**128`
                    Self(Uint::<U>::ZERO)
                }
            },
        })
    }

    generate_unchecked!(checked_from_atomics => from_atomics, args impl Into<Uint<U>>, u32);
}

impl<U, const S: usize> Decimal<U, S>
where
    Uint<U>: NextNumber + CheckedOps,
    <Uint<U> as NextNumber>::Next: From<Uint<U>> + TryInto<Uint<U>> + CheckedOps + ToString + Clone,
    U: NumberConst + Clone + PartialEq + Copy + FromStr,
{
    pub fn checked_from_ratio(
        numerator: impl Into<Uint<U>>,
        denominator: impl Into<Uint<U>>,
    ) -> StdResult<Self> {
        let numerator: Uint<U> = numerator.into();
        let denominator: Uint<U> = denominator.into();
        numerator.checked_multiply_ratio(Self::decimal_fraction(), denominator).map(Self)
    }

    pub fn from_ratio(numerator: impl Into<Uint<U>>, denominator: impl Into<Uint<U>>) -> Self {
        Self::checked_from_ratio(numerator, denominator).unwrap()
    }

    pub fn checked_mul(self, rhs: Self) -> StdResult<Self> {
        let numerator = self.0.full_mul(rhs.numerator());
        let denominator = <Uint<U> as NextNumber>::Next::from(Self::decimal_fraction());
        let next_result = numerator.checked_div(denominator)?;
        TryInto::<Uint<U>>::try_into(next_result.clone())
            .map(Self)
            .map_err(|_| StdError::overflow_conversion::<_, Uint<U>>(next_result))
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

    generate_unchecked!(checked_pow => pow, arg u32);
}

impl<U, const S: usize> Sqrt for Decimal<U, S>
where
    Decimal<U, S>: ToString,
    Uint<U>: CheckedOps + NumberConst + Sqrt + Copy + PartialOrd + PartialEq,
    U: NumberConst,
{
    fn checked_sqrt(self) -> StdResult<Self> {
        if self.0 < Uint::ZERO {
            return Err(StdError::negative_sqrt::<Self>(self));
        }
        let hundred = Uint::TEN.checked_mul(Uint::TEN)?;
        (0..=S as u32 / 2)
            .rev()
            .find_map(|i| {
                let inner_mul = hundred.checked_pow(i).unwrap();
                self.0.checked_mul(inner_mul).ok().map(|inner| {
                    let outer_mul = hundred.checked_pow(S as u32 / 2 - i).unwrap();
                    Self::raw(inner.sqrt().checked_mul(outer_mul).unwrap())
                })
            })
            .ok_or(StdError::Generic("Sqrt failed".to_string()))
    }
}

impl_base_ops!(impl Decimal with Add, add for Decimal<U, S> where sub fn checked_add);
impl_base_ops!(impl Decimal with Sub, sub for Decimal<U, S> where sub fn checked_sub);
impl_base_ops!(impl Decimal with Mul, mul for Decimal<U, S> where sub fn checked_mul);
impl_base_ops!(impl Decimal with Div, div for Decimal<U, S> where sub fn checked_div);

impl<U, const S: usize> Display for Decimal<U, S>
where
    Uint<U>: CheckedOps + Display + Copy,
    U: NumberConst + PartialEq + PartialOrd,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let decimals = Self::decimal_fraction();
        let whole = (self.0) / decimals;
        let fractional = (self.0).checked_rem(decimals).unwrap();

        if whole < Uint::ZERO || fractional < Uint::ZERO {
            write!(f, "-")?;
        }

        if fractional.is_zero() {
            write!(f, "{whole}")
        } else {
            let fractional_string = format!("{:0>padding$}", fractional.abs(), padding = S);
            f.write_str(&whole.abs().to_string())?;
            f.write_char('.')?;
            f.write_str(&fractional_string.trim_end_matches('0').replace("-", ""))?;
            Ok(())
        }
    }
}

impl<U, const S: usize> FromStr for Decimal<U, S>
where
    Uint<U>: CheckedOps + FromStr + Display,
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
            // let fractional_factor = Uint::<U>::new(10u128.pow(exp));
            let fractional_factor = Uint::TEN.checked_pow(exp as u32).unwrap();

            // This multiplication can't overflow because
            // fractional < 10^DECIMAL_PLACES && fractional_factor <= 10^DECIMAL_PLACES
            let fractional_part = Uint::from(fractional).checked_mul(fractional_factor).unwrap();

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

pub type Decimal128 = Decimal<u128, 18>;
pub type SignedDecimal128 = Decimal<i128, 18>;

#[test]
fn t1() {
    assert_eq!(Decimal128::percent(50_u128), Decimal128::raw(500_000_000_000_000_000_u128.into()));
    assert_eq!(Decimal128::permille(50_u128), Decimal128::raw(5_000_000_000_000_000_u128.into()));

    let val = SignedDecimal128::from_str("-1.35").unwrap();
    println!("{val}");

    let c = SignedDecimal128::percent(-50_i128);
    println!("{c}");

    let c = SignedDecimal128::permille(-50_i128);
    println!("{c}");
}
