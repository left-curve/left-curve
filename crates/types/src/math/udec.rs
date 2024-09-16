use {
    crate::{
        forward_ref_binop_decimal, forward_ref_op_assign_decimal, generate_decimal,
        impl_all_ops_and_assign, impl_assign_number, impl_number, Decimal, Fraction, Inner,
        MultiplyRatio, NextNumber, NonZero, Number, NumberConst, Sign, StdError, StdResult, Uint,
    },
    bnum::types::U256,
    borsh::{BorshDeserialize, BorshSerialize},
    forward_ref::{forward_ref_binop, forward_ref_op_assign},
    serde::{de, ser},
    std::{
        cmp::Ordering,
        fmt::{self, Display, Write},
        marker::PhantomData,
        ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign},
        str::FromStr,
    },
};

// ------------------------------- generic type --------------------------------

#[derive(
    BorshSerialize, BorshDeserialize, Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord,
)]
pub struct Udec<U, const S: u32>(pub(crate) Uint<U>);

impl<U, const S: u32> Udec<U, S> {
    /// Ratio between the inner integer value and the decimal value it
    /// represents.
    ///
    /// Since we use `u128` here, whose maximum value is 3.4e+38, it's possible
    /// to have at most 37 decimal places. Going higher would cause this `pow`
    /// calculation to overflow, resulting in a compile time error.
    pub const DECIMAL_FRACTION: u128 = 10u128.pow(Self::DECIMAL_PLACES);
    /// Number of decimal digits to be interpreted as decimal places.
    pub const DECIMAL_PLACES: u32 = S;

    /// Create a new [`Udec`] _without_ adding decimal places.
    ///
    /// ```rust
    /// use {
    ///     grug_types::{Udec128, Uint128},
    ///     std::str::FromStr,
    /// };
    ///
    /// let uint = Uint128::new(100);
    /// let decimal = Udec128::raw(uint);
    /// assert_eq!(decimal, Udec128::from_str("0.000000000000000100").unwrap());
    /// ```
    pub const fn raw(value: Uint<U>) -> Self {
        Self(value)
    }

    pub fn numerator(&self) -> &Uint<U> {
        &self.0
    }
}

impl<U, const S: u32> NumberConst for Udec<U, S>
where
    Uint<U>: NumberConst,
{
    const MAX: Self = Self(Uint::MAX);
    const MIN: Self = Self(Uint::MIN);
    // TODO: These two (one and ten) can be confusing. How can we make this
    // clear for users?
    const ONE: Self = Self(Uint::ONE);
    const TEN: Self = Self(Uint::TEN);
    const ZERO: Self = Self(Uint::ZERO);
}

impl<U, const S: u32> Udec<U, S>
where
    Uint<U>: From<u128>,
{
    // This can't be made `const` because of the `into` casting isn't constant.
    pub fn one() -> Self {
        Self(Self::decimal_fraction())
    }

    pub fn decimal_fraction() -> Uint<U> {
        Self::DECIMAL_FRACTION.into()
    }
}

impl<U, const S: u32> Udec<U, S>
where
    Uint<U>: Number + From<u128>,
{
    /// Create a new [`Udec`] adding decimal places.
    ///
    /// ```rust
    /// use {
    ///     grug_types::{Udec128, Uint128},
    ///     std::str::FromStr,
    /// };
    ///
    /// let uint = Uint128::new(100);
    /// let decimal = Udec128::new(uint);
    /// assert_eq!(decimal, Udec128::from_str("100.0").unwrap());
    /// ```
    pub fn new(value: impl Into<Uint<U>>) -> Self {
        Self(value.into() * Self::decimal_fraction())
    }

    pub fn new_percent(x: impl Into<Uint<U>>) -> Self {
        Self(x.into() * (Self::DECIMAL_FRACTION / 100).into())
    }

    pub fn new_permille(x: impl Into<Uint<U>>) -> Self {
        Self(x.into() * (Self::DECIMAL_FRACTION / 1_000).into())
    }

    pub fn new_bps(x: impl Into<Uint<U>>) -> Self {
        Self(x.into() * (Self::DECIMAL_FRACTION / 1_000_000).into())
    }
}

impl<U, const S: u32> Udec<U, S>
where
    Uint<U>: NumberConst + Number + From<u128>,
{
    pub fn checked_from_atomics(
        atomics: impl Into<Uint<U>>,
        decimal_places: u32,
    ) -> StdResult<Self> {
        let atomics = atomics.into();

        let inner = match decimal_places.cmp(&S) {
            Ordering::Less => {
                // No overflow because decimal_places < S
                let digits = Self::DECIMAL_PLACES - decimal_places;
                let factor = Uint::<U>::TEN.checked_pow(digits)?;
                atomics.checked_mul(factor)?
            },
            Ordering::Equal => atomics,
            Ordering::Greater => {
                // No overflow because decimal_places > S
                let digits = decimal_places - Self::DECIMAL_PLACES;
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

impl<U, const S: u32> Udec<U, S>
where
    Uint<U>: MultiplyRatio + From<u128>,
{
    pub fn checked_from_ratio(
        numerator: impl Into<Uint<U>>,
        denominator: impl Into<Uint<U>>,
    ) -> StdResult<Self> {
        let numerator: Uint<U> = numerator.into();
        let denominator: Uint<U> = denominator.into();
        numerator
            .checked_multiply_ratio_floor(Self::DECIMAL_FRACTION, denominator)
            .map(Self)
    }
}

// Methods for converting one `Udec` value to another `Udec` type with a
// different word size and decimal places.
//
// We can't implement the `From` and `TryFrom` traits here, because it would
// conflict with the standard library's `impl From<T> for T`, as we can't yet
// specify that `U != OU` or `S != OS` with stable Rust.
impl<U, const S: u32> Udec<U, S>
where
    Uint<U>: NumberConst + Number,
{
    pub fn from_decimal<OU, const OS: u32>(other: Udec<OU, OS>) -> Self
    where
        Uint<U>: From<Uint<OU>>,
    {
        if OS > S {
            let adjusted_precision = Uint::<U>::TEN.checked_pow(OS - S).unwrap();
            Self(Uint::<U>::from(other.0) / adjusted_precision)
        } else {
            let adjusted_precision = Uint::<U>::TEN.checked_pow(S - OS).unwrap();
            Self(Uint::<U>::from(other.0) * adjusted_precision)
        }
    }

    pub fn try_from_decimal<OU, const OS: u32>(other: Udec<OU, OS>) -> StdResult<Self>
    where
        Uint<U>: TryFrom<Uint<OU>>,
        StdError: From<<Uint<U> as TryFrom<Uint<OU>>>::Error>,
    {
        if OS > S {
            let adjusted_precision = Uint::<U>::TEN.checked_pow(OS - S)?;
            Uint::<U>::try_from(other.0)
                .map(|val| val.checked_div(adjusted_precision))?
                .map(Self)
        } else {
            let adjusted_precision = Uint::<U>::TEN.checked_pow(S - OS)?;
            Uint::<U>::try_from(other.0)
                .map(|val| val.checked_mul(adjusted_precision))?
                .map(Self)
        }
    }
}

impl<U, const S: u32> Decimal for Udec<U, S>
where
    U: Copy + PartialEq,
    Uint<U>: Number + From<u128>,
{
    fn checked_floor(self) -> StdResult<Self> {
        // There are two ways to floor:
        // 1. inner / decimal_fraction * decimal_fraction
        // 2. inner - inner % decimal_fraction
        // Method 2 is faster because Rem is roughly as fast as or slightly
        // faster than Div, while Sub is significantly faster than Mul.
        //
        // This flooring operation in fact can never fail, because flooring an
        // unsigned decimal goes down to 0 at most. However, flooring a _signed_
        // decimal may underflow.
        Ok(Self(self.0 - self.0.checked_rem(Self::decimal_fraction())?))
    }

    fn checked_ceil(self) -> StdResult<Self> {
        let floor = self.checked_floor()?;
        if floor == self {
            Ok(floor)
        } else {
            floor.0.checked_add(Self::decimal_fraction()).map(Self)
        }
    }
}

impl<U, const S: u32> Inner for Udec<U, S> {
    type U = U;
}

impl<U, const S: u32> Sign for Udec<U, S> {
    fn abs(self) -> Self {
        self
    }

    fn is_negative(&self) -> bool {
        false
    }
}

impl<U, const S: u32> Fraction<U> for Udec<U, S>
where
    Uint<U>: Number + MultiplyRatio + Copy + From<u128>,
    Udec<U, S>: Number + Display,
{
    fn numerator(&self) -> Uint<U> {
        self.0
    }

    fn denominator() -> NonZero<Uint<U>> {
        // We know the decimal fraction is non-zero, because it's defined as a
        // power (10^S), so we can safely wrap it in `NonZero` without checking.
        NonZero(Self::decimal_fraction())
    }

    fn inv(&self) -> StdResult<Self> {
        if self.is_zero() {
            Err(StdError::division_by_zero(self))
        } else {
            Self::checked_from_ratio(Self::decimal_fraction(), self.0)
        }
    }
}

impl<U, const S: u32> Number for Udec<U, S>
where
    U: NumberConst + Number + Copy + PartialEq + PartialOrd + Display,
    Uint<U>: NextNumber + From<u128>,
    <Uint<U> as NextNumber>::Next: Number + Copy + ToString,
{
    fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    fn abs(self) -> Self {
        // `Udec` represents an unsigned decimal number, so the absolute
        // value is sipmly itself.
        self
    }

    fn checked_add(self, other: Self) -> StdResult<Self> {
        self.0.checked_add(other.0).map(Self)
    }

    fn checked_sub(self, other: Self) -> StdResult<Self> {
        self.0.checked_sub(other.0).map(Self)
    }

    fn checked_mul(self, other: Self) -> StdResult<Self> {
        let next_result = self
            .0
            .checked_full_mul(*other.numerator())?
            .checked_div(Self::decimal_fraction().into())?;
        next_result
            .try_into()
            .map(Self)
            .map_err(|_| StdError::overflow_conversion::<_, Uint<U>>(next_result))
    }

    fn checked_div(self, other: Self) -> StdResult<Self> {
        Udec::checked_from_ratio(*self.numerator(), *other.numerator())
    }

    fn checked_rem(self, other: Self) -> StdResult<Self> {
        self.0.checked_rem(other.0).map(Self)
    }

    fn checked_pow(mut self, mut exp: u32) -> StdResult<Self> {
        if exp == 0 {
            return Ok(Self::one());
        }

        let mut y = Udec::one();
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
        self.checked_mul(y)
    }

    fn checked_sqrt(self) -> StdResult<Self> {
        // With the current design, U should be only unsigned number.
        // Leave this safety check here for now.
        if self.0 < Uint::ZERO {
            return Err(StdError::negative_sqrt::<Self>(self));
        }
        let hundred = Uint::TEN.checked_mul(Uint::TEN)?;
        (0..=S / 2)
            .rev()
            .find_map(|i| -> Option<StdResult<Self>> {
                let inner_mul = match hundred.checked_pow(i) {
                    Ok(val) => val,
                    Err(err) => return Some(Err(err)),
                };
                self.0.checked_mul(inner_mul).ok().map(|inner| {
                    let outer_mul = Uint::TEN.checked_pow(S / 2 - i)?;
                    Ok(Self::raw(inner.checked_sqrt()?.checked_mul(outer_mul)?))
                })
            })
            .transpose()?
            .ok_or(StdError::Generic("sqrt failed".to_string())) // TODO: add a StdError variant to handle this?
    }

    fn wrapping_add(self, other: Self) -> Self {
        Self(self.0.wrapping_add(other.0))
    }

    fn wrapping_sub(self, other: Self) -> Self {
        Self(self.0.wrapping_sub(other.0))
    }

    fn wrapping_mul(self, other: Self) -> Self {
        Self(self.0.wrapping_mul(other.0))
    }

    fn wrapping_pow(self, other: u32) -> Self {
        Self(self.0.wrapping_pow(other))
    }

    fn saturating_add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }

    fn saturating_mul(self, other: Self) -> Self {
        Self(self.0.saturating_mul(other.0))
    }

    fn saturating_pow(self, other: u32) -> Self {
        Self(self.0.saturating_pow(other))
    }
}

impl<U, const S: u32> Display for Udec<U, S>
where
    U: Display,
    Uint<U>: Number + Copy + From<u128>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let decimals = Self::DECIMAL_FRACTION.into();
        let whole = (self.0) / decimals;
        let fractional = (self.0).checked_rem(decimals).unwrap();

        if fractional.is_zero() {
            write!(f, "{whole}")?;
        } else {
            let fractional_string = format!("{:0>padding$}", fractional.0, padding = S as usize);
            f.write_str(&whole.to_string())?;
            f.write_char('.')?;
            f.write_str(&fractional_string.trim_end_matches('0').replace('-', ""))?;
        }

        Ok(())
    }
}

impl<U, const S: u32> FromStr for Udec<U, S>
where
    Uint<U>: NumberConst + Number + Display + FromStr + From<u128>,
{
    type Err = StdError;

    /// Converts the decimal string to a Udec
    /// Possible inputs: "1.23", "1", "000012", "1.123000000"
    /// Disallowed: "", ".23"
    ///
    /// This never performs any kind of rounding.
    /// More than DECIMAL_PLACES fractional digits, even zeros, result in an error.
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut parts_iter = input.split('.');

        let mut atomics = parts_iter
            .next()
            .unwrap() // split always returns at least one element
            .parse::<Uint<U>>()
            .map_err(|_| StdError::generic_err("error parsing whole"))?
            .checked_mul(Self::decimal_fraction())
            .map_err(|_| StdError::generic_err("value too big"))?;

        if let Some(fractional_part) = parts_iter.next() {
            let fractional = fractional_part
                .parse::<Uint<U>>()
                .map_err(|_| StdError::generic_err("error parsing fractional"))?;
            let exp = (S.checked_sub(fractional_part.len() as u32)).ok_or_else(|| {
                StdError::generic_err(format!("cannot parse more than {} fractional digits", S))
            })?;
            debug_assert!(exp <= S);

            let fractional_factor = Uint::TEN.checked_pow(exp).unwrap();

            // This multiplication can't overflow because
            // fractional < 10^DECIMAL_PLACES && fractional_factor <= 10^DECIMAL_PLACES
            let fractional_part = fractional.checked_mul(fractional_factor).unwrap();

            // for negative numbers, we need to subtract the fractional part
            atomics = atomics
                .checked_add(fractional_part)
                .map_err(|_| StdError::generic_err("value too big"))?;
        }

        if parts_iter.next().is_some() {
            return Err(StdError::generic_err("unexpected number of dots"));
        }

        Ok(Udec(atomics))
    }
}

impl<U, const T: u32> ser::Serialize for Udec<U, T>
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

impl<'de, U, const S: u32> de::Deserialize<'de> for Udec<U, S>
where
    Udec<U, S>: FromStr,
    <Udec<U, S> as FromStr>::Err: Display,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_str(DecimalVisitor::new())
    }
}

struct DecimalVisitor<U, const S: u32> {
    _marker: PhantomData<U>,
}

impl<U, const S: u32> DecimalVisitor<U, S> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<'de, U, const S: u32> de::Visitor<'de> for DecimalVisitor<U, S>
where
    Udec<U, S>: FromStr,
    <Udec<U, S> as FromStr>::Err: Display,
{
    type Value = Udec<U, S>;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("string-encoded decimal")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Udec::from_str(v).map_err(E::custom)
    }
}

impl_number!(impl Udec with Add, add for Udec<U, S> where sub fn checked_add);
impl_number!(impl Udec with Sub, sub for Udec<U, S> where sub fn checked_sub);
impl_number!(impl Udec with Mul, mul for Udec<U, S> where sub fn checked_mul);
impl_number!(impl Udec with Div, div for Udec<U, S> where sub fn checked_div);
impl_number!(impl Udec with Rem, rem for Udec<U, S> where sub fn checked_rem);

impl_assign_number!(impl Udec with AddAssign, add_assign for Udec<U, S> where sub fn checked_add);
impl_assign_number!(impl Udec with SubAssign, sub_assign for Udec<U, S> where sub fn checked_sub);
impl_assign_number!(impl Udec with MulAssign, mul_assign for Udec<U, S> where sub fn checked_mul);
impl_assign_number!(impl Udec with DivAssign, div_assign for Udec<U, S> where sub fn checked_div);
impl_assign_number!(impl Udec with RemAssign, rem_assign for Udec<U, S> where sub fn checked_rem);

forward_ref_binop_decimal!(impl Add, add for Udec<U, S>, Udec<U, S>);
forward_ref_binop_decimal!(impl Sub, sub for Udec<U, S>, Udec<U, S>);
forward_ref_binop_decimal!(impl Mul, mul for Udec<U, S>, Udec<U, S>);
forward_ref_binop_decimal!(impl Div, div for Udec<U, S>, Udec<U, S>);
forward_ref_binop_decimal!(impl Rem, rem for Udec<U, S>, Udec<U, S>);

forward_ref_op_assign_decimal!(impl AddAssign, add_assign for Udec<U, S>, Udec<U, S>);
forward_ref_op_assign_decimal!(impl SubAssign, sub_assign for Udec<U, S>, Udec<U, S>);
forward_ref_op_assign_decimal!(impl MulAssign, mul_assign for Udec<U, S>, Udec<U, S>);
forward_ref_op_assign_decimal!(impl DivAssign, div_assign for Udec<U, S>, Udec<U, S>);
forward_ref_op_assign_decimal!(impl RemAssign, rem_assign for Udec<U, S>, Udec<U, S>);

impl<IntoUintU, U, const S: u32> Mul<IntoUintU> for Udec<U, S>
where
    U: Number,
    IntoUintU: Into<Uint<U>>,
{
    type Output = Self;

    fn mul(self, rhs: IntoUintU) -> Self::Output {
        Self::raw(self.0 * rhs.into())
    }
}

impl<IntoUintU, U, const S: u32> Div<IntoUintU> for Udec<U, S>
where
    U: Number,
    IntoUintU: Into<Uint<U>>,
{
    type Output = Self;

    fn div(self, rhs: IntoUintU) -> Self::Output {
        Self::raw(self.0 / rhs.into())
    }
}

// ------------------------------ concrete types -------------------------------

generate_decimal!(
    name = Udec128,
    inner_type = u128,
    decimal_places = 18,
    from_dec = [],
    doc = "128-bit unsigned fixed-point number with 18 decimal places.",
);

generate_decimal!(
    name = Udec256,
    inner_type = U256,
    decimal_places = 18,
    from_dec = [Udec128],
    doc = "256-bit unsigned fixed-point number with 18 decimal places.",
);

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {

    use {
        super::*,
        crate::{JsonDeExt, JsonSerExt, Signed, Uint128, Uint256},
        fmt::Debug,
    };

    fn dec<U, const S: u32>(str: &str) -> Udec<U, S>
    where
        Uint<U>: NumberConst + Number + Display + FromStr + From<u128>,
    {
        Udec::from_str(str).unwrap()
    }

    #[test]
    fn decimal_from_decimal256_works() {
        let too_big = Udec256::raw(Uint256::from(Uint128::MAX) + Uint256::ONE);

        assert!(matches!(
            Udec128::try_from(too_big).unwrap_err(),
            StdError::OverflowConversion { .. }
        ));

        let just_right = Udec256::raw(Uint256::from(Uint128::MAX));
        assert_eq!(Udec128::try_from(just_right).unwrap(), Udec128::MAX);

        assert_eq!(Udec128::try_from(Udec256::ZERO).unwrap(), Udec128::ZERO);
        assert_eq!(Udec128::try_from(Udec256::ONE).unwrap(), Udec128::ONE);
    }

    /// `derive_type`
    ///
    /// Allow compiler to derive the type of a variable,
    /// which is necessary for the test functions.
    fn dt<T>(_: T, _: T) {}

    /// `built_type`
    ///
    ///  Allow compiler to derive the type of a variable, and return right.
    fn bt<T>(_: T, ret: T) -> T {
        ret
    }

    fn decimal_places<U, const S: u32>(_: Udec<U, S>) -> u32 {
        S
    }

    fn decimal_fraction<U, const S: u32>(_: Udec<U, S>) -> Uint<U>
    where
        Uint<U>: From<u128>,
    {
        Udec::<U, S>::decimal_fraction()
    }

    /// `derive_types`
    ///
    ///  Allow compiler to derive the types of multiple variables
    macro_rules! dts{
        ($u: expr, $($p:expr),* ) =>
         {
            $(dt($u, $p);)*
         }
    }

    /// Macro for unit tests for Udec.
    /// Is not possible to use [`test_case::test_case`] because the arguments types can are different.
    /// Also `Udec<U>` is different for each test case.
    ///
    /// The macro set as first parameter of the callback function `Uint::ZERO`, so the compiler can derive the type
    /// (see [`derive_type`], [`derive_types`] and [`smart_assert`] ).
    macro_rules! dtest {
        // Multiple args
        (
            $name:ident,
            [$($p128:expr),*],
            [$($p256:expr),*]
            $(attrs = $(#[$meta:meta])*)?
            => $test_fn:expr) => {
            paste::paste! {
                #[test]
                $($(#[$meta])*)?
                fn [<$name _udec128 >]() {
                    ($test_fn)(Udec128::ZERO, $($p128),*);
                }

                #[test]
                $($(#[$meta])*)?
                fn [<$name _udec256 >]() {
                    ($test_fn)(Udec256::ZERO, $($p256),*);
                }

            }
        };
        // No args
        (
            $name:ident,
            $(attrs = $(#[$meta:meta])*)?
            => $test_fn:expr) => {
                dtest!($name, [], [] $(attrs = $(#[$meta])*)? => $test_fn);
        };
        // Same args
        (
            $name:ident,
            [$($p:expr),*]
            $(attrs = $(#[$meta:meta])*)?
            => $test_fn:expr) => {
                dtest!($name, [$($p),*], [$($p),*] $(attrs = $(#[$meta])*)? => $test_fn);
        };
    }

    dtest!( new,
     => |_0d| {
        // raw
        let raw = bt(_0d, Udec::raw(Uint::from(100_u64)));
        assert_eq!(raw.0, Uint::from(100_u64));

        // new
        let new = bt(_0d, Udec::new(100_u128));
        assert_eq!(new, Udec::from_str("100.0").unwrap());

        // zero
        assert_eq!(_0d, Udec::from_str("0.0").unwrap());

        // one
        let one = bt(_0d, Udec::one());
        assert_eq!(one, Udec::from_str("1.0").unwrap());

        // percent
        let percent = bt(_0d, Udec::new_percent(1_u128));
        assert_eq!(percent, Udec::from_str("0.01").unwrap());

        // permille
        let permille = bt(_0d, Udec::new_permille(1_u128));
        assert_eq!(permille, Udec::from_str("0.001").unwrap());

        // bps
        let bps = bt(_0d, Udec::new_bps(1_u128));
        assert_eq!(bps, Udec::from_str("0.000001").unwrap());
     }
    );

    dtest!( from_signed,
        => |_0d| {

            let raw = bt(_0d, Udec::from_str("100.0").unwrap());
            let neg = Signed::new_negative(raw);

            // Invalid negative
            let res = bt(Ok(_0d), Udec::try_from(neg));
            assert!(matches!(res, Err(StdError::OverflowConversion { .. })));

            // Valid Positive
            let pos = Signed::new_positive(raw);
            dt(_0d, Udec::try_from(pos).unwrap());

            // -0 works
            let neg = Signed::new_negative(_0d);
            dt(_0d, Udec::try_from(neg).unwrap());
        }
    );

    dtest!( atomics,
        ["0.340282366920938463", 39],
        ["0.115792089237316195", 78]
        attrs = #[allow(clippy::useless_conversion)]
        => |_0d, max_str, max_digits| {
            let one = Udec::one();
            let two = Udec::new(2_u128);
            dts!(_0d, one, two);

            fn check_atomics<U, IU,  const S: u32>(compare: Udec<U, S>, cases: &[(IU, u32)]) where
                U: PartialEq + Debug,
                Uint<U>: NumberConst + Number + From<u128>,
                IU: Into<Uint<U>> + Copy
             {
                for (atomics, decimal_places) in cases {
                    let dec = Udec::checked_from_atomics(*atomics, *decimal_places).unwrap();
                    assert_eq!(dec, compare);
                }
            }

            check_atomics(
                one,
                &[
                    (1, 0),
                    (10, 1),
                    (100, 2),
                    (10_u128.pow(18), 18),
                    (10_u128.pow(20), 20)
                ]
            );

            check_atomics(
                two,
                &[
                    (2, 0),
                    (20, 1),
                    (200, 2),
                    (2 * 10_u128.pow(18), 18),
                    (2 * 10_u128.pow(20), 20)
                ]
            );

            // Cuts decimal digits (max 18)

            fn check_atomics_with_str<U, IU,  const S: u32>(phantom: Udec<U, S>, cases: &[(IU, u32, &str)]) where
                U: PartialEq + Debug + FromStr + Display,
                <U as FromStr>::Err: Display,
                Uint<U>: NumberConst + Number + From<u128>,
                IU: Into<Uint<U>> + Copy
            {
                for (atomics, decimal_places, str) in cases {
                    let dec = Udec::checked_from_atomics(*atomics, *decimal_places).unwrap();
                    let compare = Udec::from_str(str).unwrap();
                    dts!(&phantom, &dec, &compare);
                    assert_eq!(dec, compare);
                }
            }

            let inner_max = bt(_0d.0, Uint::MAX);

            check_atomics_with_str(_0d,
                &[
                    (4321u128.into(), 20, "0.000000000000000043"),
                    (6789u128.into(), 20, "0.000000000000000067"),
                    (inner_max.0, max_digits, max_str)
                ]
            );

            // Overflow is only possible with digits < 18
            let result = bt(Ok(_0d), Udec::checked_from_atomics(inner_max, 17));
            assert!(matches!(result, Err(StdError::OverflowMul { .. })));
        }
    );

    dtest!( from_ratio,
        => |_0d| {
            let _1d = Udec::one();
            let _1_5d = Udec::from_str("1.5").unwrap();
            let _0_125d = Udec::from_str("0.125").unwrap();
            let max = Udec::MAX;
            dts!(_0d, _0_125d, _1d, _1_5d, max);

            // 1.0
            assert_eq!(Udec::checked_from_ratio(1_u128, 1_u128).unwrap(), _1d);
            assert_eq!(Udec::checked_from_ratio(53_u128, 53_u128).unwrap(), _1d);
            assert_eq!(Udec::checked_from_ratio(125_u128, 125_u128).unwrap(), _1d);

            // 1.5
            assert_eq!(Udec::checked_from_ratio(3u128, 2_u128).unwrap(), _1_5d);
            assert_eq!(Udec::checked_from_ratio(150_u128, 100_u128).unwrap(), _1_5d);
            assert_eq!(Udec::checked_from_ratio(333_u128, 222_u128).unwrap(), _1_5d);
            // 0.125

            assert_eq!(Udec::checked_from_ratio(1_u128, 8_u128).unwrap(), _0_125d);
            assert_eq!(Udec::checked_from_ratio(125_u128, 1000_u128).unwrap(), _0_125d);

            // 1/3 (result floored)
            assert_eq!(
                Udec::checked_from_ratio(1u64, 3u64).unwrap(),
                bt(_0d, Udec::from_str("0.333333333333333333").unwrap())
            );

            // 2/3 (result floored)
            assert_eq!(
                Udec::checked_from_ratio(2u64, 3u64).unwrap(),
                bt(_0d, Udec::from_str("0.666666666666666666").unwrap())
            );

            // large inputs
            let uint_max = Uint::MAX;
            let precision = Uint::TEN.checked_pow(decimal_places(_0d)).unwrap();
            dts!(_0d.0, uint_max, precision);
            assert_eq!(Udec::checked_from_ratio(0_u128, uint_max).unwrap(), _0d);
            assert_eq!(Udec::checked_from_ratio(uint_max, uint_max).unwrap(), _1d);
            assert_eq!(
                Udec::checked_from_ratio(uint_max / precision, 1_u128).unwrap(),
                bt(_0d, Udec::new(uint_max / precision))
            );

            // 0 denominator
            let result = bt(Ok(_0d), Udec::checked_from_ratio(1_u128, 0_u128));
            assert!(matches!(result, Err(StdError::DivisionByZero { .. })));

            // overflow conversion
            let result = bt(Ok(_0d), Udec::checked_from_ratio(uint_max, 1_u128));
            assert!(matches!(result, Err(StdError::OverflowConversion { .. })));
        }
    );

    dtest!( fraction,
        => |_0d| {

            let val = bt(_0d, Udec::from_str("12.34").unwrap());
            let decimal_places = bt(_0d.0, Uint::TEN).checked_pow(decimal_places(_0d) - 2).unwrap();
            let numerator = bt(_0d.0, Uint::from(1234_u64)) * decimal_places;
            assert_eq!(*val.numerator(), numerator);

        }
    );

    dtest!( from_str,
        ["340282366920938463463.374607431768211455"],
        ["115792089237316195423570985008687907853269984665640564039457.584007913129639935"]
        => |_0d, max_str| {
            // Integers
            assert_eq!(Udec::from_str("0").unwrap(), _0d);
            assert_eq!(Udec::from_str("1").unwrap(), bt(_0d, Udec::new_percent(100_u128)));
            assert_eq!(Udec::from_str("5").unwrap(), bt(_0d, Udec::new_percent(500_u128)));
            assert_eq!(Udec::from_str("42").unwrap(), bt(_0d, Udec::new_percent(4200_u128)));
            assert_eq!(Udec::from_str("000").unwrap(), _0d);
            assert_eq!(Udec::from_str("001").unwrap(), bt(_0d, Udec::new_percent(100_u128)));
            assert_eq!(Udec::from_str("005").unwrap(), bt(_0d, Udec::new_percent(500_u128)));
            assert_eq!(Udec::from_str("0042").unwrap(), bt(_0d, Udec::new_percent(4200_u128)));

            // Decimals
            assert_eq!(Udec::from_str("1.0").unwrap(), bt(_0d, Udec::new_percent(100_u128)));
            assert_eq!(Udec::from_str("1.5").unwrap(), bt(_0d, Udec::new_percent(150_u128)));
            assert_eq!(Udec::from_str("0.5").unwrap(), bt(_0d, Udec::new_percent(50_u128)));
            assert_eq!(Udec::from_str("0.123").unwrap(), bt(_0d, Udec::new_permille(123_u128)));
            assert_eq!(Udec::from_str("40.00").unwrap(), bt(_0d, Udec::new_percent(4000_u128)));
            assert_eq!(Udec::from_str("04.00").unwrap(), bt(_0d, Udec::new_percent(400_u128)));
            assert_eq!(Udec::from_str("00.40").unwrap(), bt(_0d, Udec::new_percent(40_u128)));
            assert_eq!(Udec::from_str("00.04").unwrap(), bt(_0d, Udec::new_percent(4_u128)));

             // Can handle DECIMAL_PLACES fractional digits
            assert_eq!(
                bt(_0d, Udec::from_str("7.123456789012345678").unwrap()),
                Udec::raw(Uint::from(7123456789012345678u128))
            );
            assert_eq!(
                bt(_0d, Udec::from_str("7.999999999999999999").unwrap()),
                Udec::raw(Uint::from(7999999999999999999u128))
            );

            // Works for documented max value
            assert_eq!(
                bt(_0d, Udec::from_str(max_str).unwrap()),
                Udec::MAX
            );
        }
    );

    dtest!( from_str_errors,
        => |_0d| {
            assert!(matches!(bt(Ok(_0d), Udec::from_str("")), Err(StdError::Generic(err)) if err == "error parsing whole"));
            assert!(matches!(bt(Ok(_0d), Udec::from_str(" ")), Err(StdError::Generic(err)) if err == "error parsing whole"));
            assert!(matches!(bt(Ok(_0d), Udec::from_str("-1")), Err(StdError::Generic(err)) if err == "error parsing whole"));

            assert!(matches!(bt(Ok(_0d), Udec::from_str("1.")), Err(StdError::Generic(err)) if err == "error parsing fractional"));
            assert!(matches!(bt(Ok(_0d), Udec::from_str("1. ")), Err(StdError::Generic(err)) if err == "error parsing fractional"));
            assert!(matches!(bt(Ok(_0d), Udec::from_str("1.e")), Err(StdError::Generic(err)) if err == "error parsing fractional"));
            assert!(matches!(bt(Ok(_0d), Udec::from_str("1.2e3")), Err(StdError::Generic(err)) if err == "error parsing fractional"));

            let digits = decimal_places(_0d);
            assert!( matches!(
                bt(Ok(_0d), Udec::from_str("1.1234567890123456789")),
                Err(StdError::Generic(err)) if err == format!("cannot parse more than {digits} fractional digits")
            ));
            assert!( matches!(
                bt(Ok(_0d), Udec::from_str("1.0000000000000000000")),
                Err(StdError::Generic(err)) if err == format!("cannot parse more than {digits} fractional digits")
            ));

            assert!(matches!(bt(Ok(_0d), Udec::from_str("1.2.3")), Err(StdError::Generic(err)) if err == "unexpected number of dots"));
            assert!(matches!(bt(Ok(_0d), Udec::from_str("1.2.3.4")), Err(StdError::Generic(err)) if err == "unexpected number of dots"));

            // Uint::MAX / decimal_fraction + 1
            let over_max = bt(_0d.0, bt(_0d.0,Uint::MAX) / decimal_fraction(_0d) + bt(_0d.0, Uint::ONE));

            // Integer
            assert!(matches!(bt(Ok(_0d), Udec::from_str(&over_max.to_string())), Err(StdError::Generic(err)) if err == "value too big"));

            // Decimal
            assert!(matches!(bt(Ok(_0d), Udec::from_str(&format!("{over_max}.0"))), Err(StdError::Generic(err)) if err == "value too big"));
            assert!(matches!(bt(Ok(_0d), Udec::from_str(&format!("{over_max}.123"))), Err(StdError::Generic(err)) if err == "value too big"));
        }
    );

    dtest!( to_string,
        => |_0d,| {

            assert_eq!(bt(_0d, Udec::ZERO).to_string(), "0");
            assert_eq!(bt(_0d, Udec::one()).to_string(), "1");
            assert_eq!(bt(_0d, Udec::new_percent(500_u64)).to_string(), "5");

            // Decimals
            assert_eq!(bt(_0d, Udec::new_percent(125_u64)).to_string(), "1.25");
            assert_eq!(bt(_0d, Udec::new_percent(42638_u64)).to_string(), "426.38");
            assert_eq!(bt(_0d, Udec::new_percent(3_u64)).to_string(), "0.03");
            assert_eq!(bt(_0d, Udec::new_permille(987_u64)).to_string(), "0.987");

            for i in 0..18 {
                let dec = bt(_0d, Udec::raw(10_u64.pow(i).into()));
                assert_eq!(dec.to_string(), format!("0.{}1", "0".repeat(18 - 1 - i as usize)));

            }
        }
    );

    dtest!( serialize_serde,
        => |_0d| {
            assert_eq!(bt(_0d, Udec::ZERO).to_json_vec().unwrap(), br#""0""#);
            assert_eq!(bt(_0d, Udec::one()).to_json_vec().unwrap(), br#""1""#);
            assert_eq!(bt(_0d, Udec::new_percent(8_u64)).to_json_vec().unwrap(), br#""0.08""#);
            assert_eq!(bt(_0d, Udec::new_percent(87_u64)).to_json_vec().unwrap(), br#""0.87""#);
            assert_eq!(bt(_0d, Udec::new_percent(876_u64)).to_json_vec().unwrap(), br#""8.76""#);
            assert_eq!(bt(_0d, Udec::new_percent(8765_u64)).to_json_vec().unwrap(), br#""87.65""#);
    });

    dtest!( deserialize_serde,
        => |_0d| {
            assert_eq!(bt(_0d, br#""0""#.deserialize_json().unwrap()), Udec::ZERO);
            assert_eq!(bt(_0d, br#""1""#.deserialize_json().unwrap()),  Udec::one());
            assert_eq!(bt(_0d, br#""0.08""#.deserialize_json().unwrap()), Udec::new_percent(8_u64));
            assert_eq!(bt(_0d, br#""0.87""#.deserialize_json().unwrap()), Udec::new_percent(87_u64));
            assert_eq!(bt(_0d, br#""8.76""#.deserialize_json().unwrap()), Udec::new_percent(876_u64));
            assert_eq!(bt(_0d, br#""87.65""#.deserialize_json().unwrap()), Udec::new_percent(8765_u64));
        }
    );

    dtest!( is_zero,
        => |_0d| {
            assert!(bt(_0d, Udec::ZERO).is_zero());
            assert!(bt(_0d, Udec::new_percent(0_u64)).is_zero());
            assert!(bt(_0d, Udec::new_permille(0_u64)).is_zero());

            assert!(!bt(_0d, Udec::one()).is_zero());
            assert!(!bt(_0d, Udec::new_percent(1_u64)).is_zero());
            assert!(!bt(_0d, Udec::new_permille(1_u64)).is_zero());
        }
    );

    dtest!( inv,
        => |_0d| {
            // d = 1
            assert_eq!(bt(_0d, Udec::new(1_u128)).inv().unwrap(), Udec::new(1_u128));

            // d = 0
            assert!(matches!(_0d.inv(), Err(StdError::DivisionByZero { .. })));

            // d > 1 exact
            assert_eq!(bt(_0d, Udec::new(2_u128)).inv().unwrap(), Udec::from_str("0.5").unwrap());
            assert_eq!(bt(_0d, Udec::new(20_u128)).inv().unwrap(), Udec::from_str("0.05").unwrap());
            assert_eq!(bt(_0d, Udec::new(200_u128)).inv().unwrap(), Udec::from_str("0.005").unwrap());
            assert_eq!(bt(_0d, Udec::new(2000_u128)).inv().unwrap(), Udec::from_str("0.0005").unwrap());

            // d > 1 rounded
            assert_eq!(bt(_0d, Udec::new(3_u128)).inv().unwrap(), Udec::from_str("0.333333333333333333").unwrap());
            assert_eq!(bt(_0d, Udec::new(6_u128)).inv().unwrap(), Udec::from_str("0.166666666666666666").unwrap());

            // d < 1 exact
            assert_eq!(bt(_0d, Udec::new_percent(50_u128)).inv().unwrap(), Udec::from_str("2").unwrap());
            assert_eq!(bt(_0d, Udec::new_percent(5_u128)).inv().unwrap(), Udec::from_str("20").unwrap());
            assert_eq!(bt(_0d, Udec::new_permille(5_u128)).inv().unwrap(), Udec::from_str("200").unwrap());
            assert_eq!(bt(_0d, Udec::new_bps(500_u128)).inv().unwrap(), Udec::from_str("2000").unwrap());
        }
    );

    dtest!( add,
        attrs = #[allow(clippy::op_ref)]
        => |_0d| {
            let _1d =  Udec::one();
            let _2d = Udec::new(2_u128);
            let _3d = Udec::new(3_u128);
            let _1_5d = Udec::from_str("1.5").unwrap();
            let max = Udec::MAX;
            dts!(_0d, _1d, _2d, _3d, _1_5d, max);
            assert_eq!(_1_5d.0, decimal_fraction(_0d) * Uint::new(3_u128) / Uint::new(2_u128));

            assert_eq!(bt(_0d, Udec::new_percent(5_u128)) + bt(_0d, Udec::new_percent(4_u128)), Udec::new_percent(9_u128));
            assert_eq!(bt(_0d, Udec::new_percent(5_u128)) + _0d, Udec::new_percent(5_u128));
            assert_eq!(_0d + _0d, _0d);

            // works for refs
            assert_eq!(_1d + _2d, _3d);
            assert_eq!(&_1d + _2d, _3d);
            assert_eq!(_1d + &_2d, _3d);
            assert_eq!(&_1d + &_2d, _3d);

            // assign
            let mut a = _0d;
            a += _1d;
            assert_eq!(a, _1d);

            // works for refs
            let mut a = _2d;
            a += &_3d;
            assert_eq!(a, Udec::new(5_u128));

            // checked_add overflow
            assert!(matches!(max.checked_add(_1d), Err(StdError::OverflowAdd { .. })));
        }
    );

    dtest!( add_overflow_panic,
        attrs = #[should_panic(expected = "addition overflow")]
        => |_0d| {
            let max = Udec::MAX;
            let _1d = Udec::one();
            dts!(_0d, max, _1d);

            // overflow
            let _ = max + _1d;
        }
    );

    dtest!( sub,
        attrs = #[allow(clippy::op_ref)]
        => |_0d| {
            let _0_5d = Udec::new_percent(50_u128);
            let _1d =  Udec::one();
            let _2d = Udec::new(2_u128);
            let _3d = Udec::new(3_u128);
            dts!(_0d, _0_5d, _1d, _2d, _3d);

            // inner
            assert_eq!(_0_5d.0, decimal_fraction(_0d) / Uint::new(2_u128));

            assert_eq!(bt(_0d, Udec::new_percent(9_u128)) - bt(_0d, Udec::new_percent(4_u128)), Udec::new_percent(5_u128));
            assert_eq!(bt(_0d, Udec::new_percent(16_u128)) - bt(_0d, Udec::new_percent(16_u128)), _0d);
            assert_eq!(_0d, _0d - _0d);

            // works for refs
            assert_eq!(_3d - _2d, _1d);
            assert_eq!(&_3d - _2d, _1d);
            assert_eq!(_3d - &_2d, _1d);
            assert_eq!(&_3d - &_2d, _1d);

            // assign
            let mut a = _3d;
            a -= _1d;
            assert_eq!(a, _2d);

            // works for refs
            let mut a = _3d;
            a -= &_1d;
            assert_eq!(a, _2d);

            // checked_sub overflow
            assert!(matches!(_0d.checked_sub(_1d), Err(StdError::OverflowSub { .. })));
        }
    );

    dtest!( sub_overflow_panic,
        attrs = #[should_panic(expected = "subtraction overflow")]
        => |_0d| {
            let _1d = Udec::one();
            let _2d = Udec::new(2_u128);
            dts!(_0d, _1d, _2d);

            // overflow
            let _ = _0d - _1d;
        }
    );

    dtest!( mul,
        attrs = #[allow(clippy::op_ref)]
        => |_0d| {
            let _0_5d = Udec::new_percent(50_u128);
            let _1d = Udec::one();
            let _2d = _1d + _1d;
            let max = Udec::MAX;
            dts!(_0d, _0_5d, _1d, _2d, max);

            // 1*x
            assert_eq!(_1d *_0d, _0d);
            assert_eq!(_1d *bt(_0d, Udec::new_percent(1_u64)), Udec::new_percent(1_u64));
            assert_eq!(_1d *bt(_0d, Udec::new_percent(10_u64)), Udec::new_percent(10_u64));
            assert_eq!(_1d *bt(_0d, Udec::new_percent(100_u64)), Udec::new_percent(100_u64));
            assert_eq!(_1d *bt(_0d, Udec::new_percent(1000_u64)), Udec::new_percent(1000_u64));
            assert_eq!(_1d * max, max);
            assert_eq!(_0d * _1d, _0d);
            assert_eq!(bt(_0d, Udec::new_percent(1_u64)) * _1d, Udec::new_percent(1_u64));
            assert_eq!(bt(_0d, Udec::new_percent(10_u64)) * _1d, Udec::new_percent(10_u64));
            assert_eq!(bt(_0d, Udec::new_percent(100_u64)) * _1d, Udec::new_percent(100_u64));
            assert_eq!(bt(_0d, Udec::new_percent(1000_u64)) * _1d, Udec::new_percent(1000_u64));
            assert_eq!(max * _1d, max);

            // 2*x
            assert_eq!(_2d * bt(_0d, Udec::new_percent(0_u64)), Udec::new_percent(0_u64));
            assert_eq!(_2d * bt(_0d, Udec::new_percent(1_u64)), Udec::new_percent(2_u64));
            assert_eq!(_2d * bt(_0d, Udec::new_percent(10_u64)), Udec::new_percent(20_u64));
            assert_eq!(_2d * bt(_0d, Udec::new_percent(100_u64)), Udec::new_percent(200_u64));
            assert_eq!(_2d * bt(_0d, Udec::new_percent(1000_u64)), Udec::new_percent(2000_u64));

            // 0.5*x
            assert_eq!(_0_5d * bt(_0d, Udec::new_percent(0_u64)), Udec::new_percent(0_u64));
            assert_eq!(_0_5d * bt(_0d, Udec::new_percent(1_u64)), Udec::new_permille(5_u64));
            assert_eq!(_0_5d * bt(_0d, Udec::new_percent(10_u64)), Udec::new_percent(5_u64));
            assert_eq!(_0_5d * bt(_0d, Udec::new_percent(100_u64)), Udec::new_percent(50_u64));
            assert_eq!(_0_5d * bt(_0d, Udec::new_percent(1000_u64)), Udec::new_percent(500_u64));
            assert_eq!(bt(_0d, Udec::new_percent(0_u64)) * _0_5d, Udec::new_percent(0_u64));
            assert_eq!(bt(_0d, Udec::new_percent(1_u64)) * _0_5d, Udec::new_permille(5_u64));
            assert_eq!(bt(_0d, Udec::new_percent(10_u64)) * _0_5d, Udec::new_percent(5_u64));
            assert_eq!(bt(_0d, Udec::new_percent(100_u64)) * _0_5d, Udec::new_percent(50_u64));
            assert_eq!(bt(_0d, Udec::new_percent(1000_u64)) * _0_5d, Udec::new_percent(500_u64));

            // move left
            let a = bt(_0d, dec("123.127726548762582"));
            assert_eq!(a * bt(_0d, dec("1")), dec("123.127726548762582"));
            assert_eq!(a * bt(_0d, dec("10")), dec("1231.27726548762582"));
            assert_eq!(a * bt(_0d, dec("100")), dec("12312.7726548762582"));
            assert_eq!(a * bt(_0d, dec("1000")), dec("123127.726548762582"));
            assert_eq!(a * bt(_0d, dec("1000000")), dec("123127726.548762582"));
            assert_eq!(a * bt(_0d, dec("1000000000")), dec("123127726548.762582"));
            assert_eq!(a * bt(_0d, dec("1000000000000")), dec("123127726548762.582"));
            assert_eq!(a * bt(_0d, dec("1000000000000000")), dec("123127726548762582"));
            assert_eq!(a * bt(_0d, dec("1000000000000000000")), dec("123127726548762582000"));
            assert_eq!(bt(_0d, dec("1")) * a, dec("123.127726548762582"));
            assert_eq!(bt(_0d, dec("10")) * a, dec("1231.27726548762582"));
            assert_eq!(bt(_0d, dec("100")) * a, dec("12312.7726548762582"));
            assert_eq!(bt(_0d, dec("1000")) * a, dec("123127.726548762582"));
            assert_eq!(bt(_0d, dec("1000000")) * a, dec("123127726.548762582"));
            assert_eq!(bt(_0d, dec("1000000000")) * a, dec("123127726548.762582"));
            assert_eq!(bt(_0d, dec("1000000000000")) * a, dec("123127726548762.582"));
            assert_eq!(bt(_0d, dec("1000000000000000")) * a, dec("123127726548762582"));
            assert_eq!(bt(_0d, dec("1000000000000000000")) * a, dec("123127726548762582000"));

            // move right
            let a = bt(_0d, dec("340282366920938463463.374607431768211455"));
            assert_eq!(a * bt(_0d, dec("1.0")), dec("340282366920938463463.374607431768211455"));
            assert_eq!(a * bt(_0d, dec("0.1")), dec("34028236692093846346.337460743176821145"));
            assert_eq!(a * bt(_0d, dec("0.01")), dec("3402823669209384634.633746074317682114"));
            assert_eq!(a * bt(_0d, dec("0.001")), dec("340282366920938463.463374607431768211"));
            assert_eq!(a * bt(_0d, dec("0.000001")), dec("340282366920938.463463374607431768"));
            assert_eq!(a * bt(_0d, dec("0.000000001")), dec("340282366920.938463463374607431"));
            assert_eq!(a * bt(_0d, dec("0.000000000001")), dec("340282366.920938463463374607"));
            assert_eq!(a * bt(_0d, dec("0.000000000000001")), dec("340282.366920938463463374"));
            assert_eq!(a * bt(_0d, dec("0.000000000000000001")), dec("340.282366920938463463"));

            // works for refs
            let a = Udec::new_percent(20_u64);
            let b = Udec::new_percent(30_u64);
            let expected = Udec::new_percent(6_u64);
            dts!(_0d, a, b, expected);
            assert_eq!(a * b, expected);
            assert_eq!(&a * b, expected);
            assert_eq!(a * &b, expected);
            assert_eq!(&a * &b, expected);

            // assign
            let mut a = _0_5d;
            a *= _2d;
            assert_eq!(a, _1d);

            // works for refs
            let mut a = _0_5d;
            a *= &_2d;
            assert_eq!(a, _1d);

            // checked_mul overflow
            assert!(matches!(max.checked_mul(_2d), Err(StdError::OverflowConversion { .. })));
        }
    );

    dtest!( mul_overflow_panic,
        attrs = #[should_panic(expected = "conversion overflow")]
        => |_0d| {
            let max = Udec::MAX;
            let _2d = Udec::new(2_u128);
            dts!(_0d, max, _2d);

            // overflow
            let _ = max * _2d;
        }
    );

    dtest!( checked_mul,
        => |_0d| {
            let test_data = [
                (_0d, _0d),
                (_0d, Udec::one()),
                (Udec::one(), _0d),
                (Udec::new_percent(10_u64), _0d),
                (Udec::new_percent(10_u64), Udec::new_percent(5_u64)),
                (Udec::MAX, Udec::one()),
                (Udec::MAX, Udec::new_percent(20_u64)),
                (Udec::new_permille(6_u64), Udec::new_permille(13_u64)),
            ];

            // The regular core::ops::Mul is our source of truth for these tests.
            for (x, y) in test_data.into_iter() {
                assert_eq!(x * y, x.checked_mul(y).unwrap());
            }
        }
    );

    dtest!( div,
        attrs = #[allow(clippy::op_ref)]
        => |_0d| {
            let _0_5d = Udec::new_percent(50_u128);
            let _1d = Udec::one();
            let _2d = _1d + _1d;
            let max = Udec::MAX;
            dts!(_0d, _0_5d, _1d, _2d, max);

            // 1/x
            assert_eq!(_1d / _1d, _1d);
            assert_eq!(_1d / bt(_0d, Udec::new_percent(1_u64)), Udec::new_percent(10000_u64));
            assert_eq!(_1d / bt(_0d, Udec::new_percent(10_u64)), Udec::new_percent(1000_u64));
            assert_eq!(_1d / bt(_0d, Udec::new_percent(100_u64)), Udec::new_percent(100_u64));
            assert_eq!(_1d / bt(_0d, Udec::new_percent(1000_u64)), Udec::new_percent(10_u64));
            assert_eq!(_0d / _1d, _0d);
            assert_eq!(bt(_0d, Udec::new_percent(1_u64)) / _1d, Udec::new_percent(1_u64));
            assert_eq!(bt(_0d, Udec::new_percent(10_u64)) / _1d, Udec::new_percent(10_u64));
            assert_eq!(bt(_0d, Udec::new_percent(100_u64)) / _1d, Udec::new_percent(100_u64));
            assert_eq!(bt(_0d, Udec::new_percent(1000_u64)) / _1d, Udec::new_percent(1000_u64));

            // 2/x
            assert_eq!(_2d / bt(_0d, Udec::new_percent(1_u64)), Udec::new_percent(20000_u64));
            assert_eq!(_2d / bt(_0d, Udec::new_percent(10_u64)), Udec::new_percent(2000_u64));
            assert_eq!(_2d / bt(_0d, Udec::new_percent(100_u64)), Udec::new_percent(200_u64));
            assert_eq!(_2d / bt(_0d, Udec::new_percent(1000_u64)), Udec::new_percent(20_u64));
            assert_eq!(_0d / _2d, _0d);
            assert_eq!(bt(_0d, Udec::new_percent(1_u64)) / _2d, Udec::new_permille(5_u64));
            assert_eq!(bt(_0d, Udec::new_percent(10_u64)) / _2d, Udec::new_percent(5_u64));
            assert_eq!(bt(_0d, Udec::new_percent(100_u64)) / _2d, Udec::new_percent(50_u64));
            assert_eq!(bt(_0d, Udec::new_percent(1000_u64)) / _2d, Udec::new_percent(500_u64));

            // 0.5/x
            assert_eq!(_0_5d / bt(_0d, Udec::new_percent(1_u64)), Udec::new_percent(5000_u64));
            assert_eq!(_0_5d / bt(_0d, Udec::new_percent(10_u64)), Udec::new_percent(500_u64));
            assert_eq!(_0_5d / bt(_0d, Udec::new_percent(100_u64)), Udec::new_percent(50_u64));
            assert_eq!(_0_5d / bt(_0d, Udec::new_percent(1000_u64)), Udec::new_percent(5_u64));
            assert_eq!(_0d / _0_5d, _0d);
            assert_eq!(bt(_0d, Udec::new_percent(1_u64)) / _0_5d, Udec::new_percent(2_u64));
            assert_eq!(bt(_0d, Udec::new_percent(10_u64)) / _0_5d, Udec::new_percent(20_u64));
            assert_eq!(bt(_0d, Udec::new_percent(100_u64)) / _0_5d, Udec::new_percent(200_u64));
            assert_eq!(bt(_0d, Udec::new_percent(1000_u64)) / _0_5d, Udec::new_percent(2000_u64));

            // Move right
            let a = bt(_0d, dec("123127726548762582"));
            assert_eq!(a / bt(_0d, dec("1")), dec("123127726548762582"));
            assert_eq!(a / bt(_0d, dec("10")), dec("12312772654876258.2"));
            assert_eq!(a / bt(_0d, dec("100")), dec("1231277265487625.82"));
            assert_eq!(a / bt(_0d, dec("1000")), dec("123127726548762.582"));
            assert_eq!(a / bt(_0d, dec("1000000")), dec("123127726548.762582"));
            assert_eq!(a / bt(_0d, dec("1000000000")), dec("123127726.548762582"));
            assert_eq!(a / bt(_0d, dec("1000000000000")), dec("123127.726548762582"));
            assert_eq!(a / bt(_0d, dec("1000000000000000")), dec("123.127726548762582"));
            assert_eq!(a / bt(_0d, dec("1000000000000000000")), dec("0.123127726548762582"));
            assert_eq!(bt(_0d, dec("1")) / a, dec("0.000000000000000008"));
            assert_eq!(bt(_0d, dec("10")) / a, dec("0.000000000000000081"));
            assert_eq!(bt(_0d, dec("100")) / a, dec("0.000000000000000812"));
            assert_eq!(bt(_0d, dec("1000")) / a, dec("0.000000000000008121"));
            assert_eq!(bt(_0d, dec("1000000")) / a, dec("0.000000000008121647"));
            assert_eq!(bt(_0d, dec("1000000000")) / a, dec("0.000000008121647560"));
            assert_eq!(bt(_0d, dec("1000000000000")) / a, dec("0.000008121647560868"));
            assert_eq!(bt(_0d, dec("1000000000000000")) / a, dec("0.008121647560868164"));
            assert_eq!(bt(_0d, dec("1000000000000000000")) / a, dec("8.121647560868164773"));

            // Move left
            let a = bt(_0d, dec("0.123127726548762582"));
            assert_eq!(a / bt(_0d, dec("1.0")), dec("0.123127726548762582"));
            assert_eq!(a / bt(_0d, dec("0.1")), dec("1.23127726548762582"));
            assert_eq!(a / bt(_0d, dec("0.01")), dec("12.3127726548762582"));
            assert_eq!(a / bt(_0d, dec("0.001")), dec("123.127726548762582"));
            assert_eq!(a / bt(_0d, dec("0.000001")), dec("123127.726548762582"));
            assert_eq!(a / bt(_0d, dec("0.000000001")), dec("123127726.548762582"));
            assert_eq!(a / bt(_0d, dec("0.000000000001")), dec("123127726548.762582"));
            assert_eq!(a / bt(_0d, dec("0.000000000000001")), dec("123127726548762.582"));
            assert_eq!(a / bt(_0d, dec("0.000000000000000001")), dec("123127726548762582"));

            assert_eq!(
                bt(_0d, Udec::new_percent(15_u64)) / bt(_0d, Udec::new_percent(60_u64)),
                Udec::new_percent(25_u64)
            );

            // works for refs
            let a = Udec::new_percent(100_u64);
            let b = Udec::new_percent(20_u64);
            let expected = Udec::new_percent(500_u64);
            dts!(_0d, a, b, expected);
            assert_eq!(a / b, expected);
            assert_eq!(&a / b, expected);
            assert_eq!(a / &b, expected);
            assert_eq!(&a / &b, expected);

            // assign
            let mut a = _2d;
            a /= _2d;
            assert_eq!(a, _1d);

            // works for refs
            let mut a = _2d;
            a /= &_2d;
            assert_eq!(a, _1d);

            // checked_div overflow
            assert!(matches!(max.checked_div(_0_5d), Err(StdError::OverflowConversion { .. })));
        }
    );

    dtest!( div_by_zero_panic,
        attrs = #[should_panic(expected = "division by zero")]
        => |_0d| {
            let _1d = Udec::one();
            dts!(_0d, _1d);

            // panic
            let _ = _1d / _0d;
        }
    );

    dtest!( div_overflow_panic,
        attrs = #[should_panic(expected = "conversion overflow")]
        => |_0d| {
            let max = Udec::MAX;
            let _0_5d = Udec::new_percent(50_u128);
            dts!(_0d, max, _0_5d);

            // overflow
            let _ = max / _0_5d;
        }
    );

    dtest!( sqrt,
        ["18446744073.709551615"],
        ["340282366920938463463374607431.768211455"]
        => |_0d, max_sqrt_str| {
            let _0_5d = Udec::new_percent(50_u128);
            let _2d = Udec::new(2_u128);
            let _4d = Udec::new(4_u128);
            let max = Udec::MAX;
            dts!( _0d, _0_5d, _2d, _4d, max);

            assert_eq!(_4d.checked_sqrt().unwrap(), _2d);
            assert_eq!(_2d.checked_sqrt().unwrap(), dec("1.414213562373095048"));
            assert_eq!(_0_5d.checked_sqrt().unwrap(), dec("0.707106781186547524"));

            assert_eq!(max.checked_sqrt().unwrap(), dec(max_sqrt_str));

            macro_rules! check_len  {
                ($dec:expr) => {
                    let sqrt = $dec.checked_sqrt().unwrap();
                    let str_sqtr = sqrt.to_string();
                    let mut str_sqtr_iter = str_sqtr.split(".");
                    let (_, decimals) = (str_sqtr_iter.next(), str_sqtr_iter.next().unwrap());

                    println!("sqrt: {}", sqrt);
                    println!("base: {}", $dec);
                    println!("len: {}", decimals.len());
                    println!("len: {}", sqrt.0);
                };
            }

            check_len!(bt(_0d, dec("401")));
            check_len!(bt(_0d, dec("4001")));
            check_len!(bt(_0d, dec("40001")));
            check_len!(bt(_0d, dec("400001")));
        }
    );

    dtest!( sqrt_lost_precision,
    [&[
        // 18 + 2 + 18 = 38  | 38 - 38 = 0
        ("41", 18),
        // 18 + 3 + 18 = 39  | (39 - 38) / 2 = 0.5 -> 1 | 18 - 1 = 17
        ("40_1", 17),
        // 18 + 4 + 18 = 40  | (40 - 38) / 2 = 1   -> 1 | 18 - 1 = 17
        ("40_01", 17),
        // 18 + 5 + 18 = 41  | (41 - 38) / 2 = 1.5 -> 2 | 18 - 2 = 16
        ("40_00_1", 16),
        // 18 + 20 + 18 = 56 | (56 - 38) / 2 = 9   -> 9 | 18 - 9 = 9
        ("40_00_00_00_00_00_00_00_00_02", 9)
    ]],
    [&[
        // 18 + 2 + 18 = 38  | (38 + 1 - 78) / 2 = 0
        ("41", 18),
        // 18 + 5 + 18 = 41  | (41 + 1 - 78) / 2 = 0
        ("40_00_1", 18),
        // 18 + 20 + 18 = 56 | (56 + 1 - 78) / 2 = 0
        ("40_00_00_00_00_00_00_00_00_02", 18),
        // 18 + 40 + 18 = 80 | (76 + 1 - 78) / 2 = 0
        ("40_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00_02", 18),
        // 18 + 42 + 18 = 80 | (78 + 1 - 78) / 2 = 1 | 18 - 1 = 17
        ("40_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00_02", 17),
        // 18 + 44 + 18 = 80 | (80 + 1 - 78) / 2 = 2 | 18 - 2 = 16
        ("40_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00_02", 16),
        // 18 + 46 + 18 = 80 | (82 + 1 - 78) / 2 = 3 | 18 - 3 = 15
        ("40_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00_02", 15)
    ]]
    => |_0d, samples: &[(&str, usize)]| {
            for (raw, decimal_size) in samples {
                let raw = raw.replace("_", "");
                let sqrt = bt(_0d, dec(&raw)).checked_sqrt().unwrap().to_string();
                let decimals = sqrt.split(".").last().unwrap();
                assert_eq!(decimals.len(), *decimal_size, "{sqrt}");
            }
        }
    );

    dtest!( pow,
        => |_0d| {
            let _1d = Udec::one();
            let max = Udec::MAX;
            dts!(_0d, _1d, max);

            for exp in 0..10 {
                assert_eq!(_1d.checked_pow(exp).unwrap(),_1d);
            }

            // This case is mathematically undefined but we ensure consistency with Rust standard types
            // https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=20df6716048e77087acd40194b233494
            assert_eq!(_0d.checked_pow(0).unwrap(), _1d);

            for exp in 1..10 {
                assert_eq!(_0d.checked_pow(exp).unwrap(), _0d);
            }

            for num in &[
                Udec::new_percent(50_u64),
                Udec::new_percent(99_u64),
                Udec::new_percent(200_u64),
            ] {
                assert_eq!(num.checked_pow(0).unwrap(), _1d)
            }

            assert_eq!(Udec::new_percent(20_u64).checked_pow(2).unwrap(), bt(_0d, Udec::new_percent(4_u64)));
            assert_eq!(Udec::new_percent(20_u64).checked_pow(3).unwrap(), bt(_0d, Udec::new_permille(8_u64)));
            assert_eq!(Udec::new(2_u64).checked_pow(4).unwrap(), bt(_0d, Udec::new(16_u64)));
            assert_eq!(Udec::new(7_u64).checked_pow(5).unwrap(), bt(_0d, Udec::new(16_807_u64)));
            assert_eq!(Udec::new(7_u64).checked_pow(8).unwrap(), bt(_0d, Udec::new(5_764_801_u64)));
            assert_eq!(Udec::new(7_u64).checked_pow(10).unwrap(), bt(_0d, Udec::new(282_475_249_u64)));
            assert_eq!(Udec::new_percent(120_u64).checked_pow(123).unwrap(), bt(_0d, Udec::raw(5_486_473_221_892_422_150_877_397_607_u128.into())));
            assert_eq!(Udec::new_percent(10_u64).checked_pow(2).unwrap(), bt(_0d, Udec::new_percent(1_u64)));
            assert_eq!(Udec::new_percent(10_u64).checked_pow(18).unwrap(), bt(_0d, Udec::raw(1_u64.into())));

            // checked_pow overflow
            assert!(matches!(max.checked_pow(2), Err(StdError::OverflowConversion { .. })));
        }
    );

    dtest!( rem,
        attrs = #[allow(clippy::op_ref)]
        => |_0d| {
            // 4.02 % 1.11 = 0.69
            assert_eq!(bt(_0d, dec("4.02")) % bt(_0d, dec("1.11")), dec("0.69"));

            // 15.25 % 4 = 3.25
            assert_eq!(bt(_0d, dec("15.25")) % bt(_0d, dec("4")), dec("3.25"));

            let a = Udec::new_percent(318_u64);
            let b = Udec::new_percent(317_u64);
            let expected = Udec::new_percent(1_u64);
            dts!(_0d, a, b, expected);

            // works for refs
            assert_eq!(a % b, expected);
            assert_eq!(a % &b, expected);
            assert_eq!(&a % b, expected);
            assert_eq!(&a % &b, expected);

            // assign works
            let mut a = bt(_0d, Udec::new_percent(17673_u64));
            a %=  Udec::new_percent(2362_u64);
            assert_eq!(a, Udec::new_percent(1139_u64)); // 176.73 % 23.62 = 11.39

            let mut a = bt(_0d, Udec::new_percent(4262_u64));
            let b = Udec::new_percent(1270_u64);
            a %= &b;
            assert_eq!(a, Udec::new_percent(452_u64)); // 42.62 % 12.7 = 4.52

            // checked_div overflow
            assert!(matches!(bt(_0d, Udec::new(777_u64)).checked_rem(_0d), Err(StdError::DivisionByZero { .. })));
        }
    );

    dtest!( rem_by_zero_panic,
        attrs = #[should_panic(expected = "division by zero")]
        => |_0d| {
            let _ = Udec::one() % _0d;
        }
    );

    dtest!( mul_into_uint,
        => |_0d| {
            let _1_5d = bt(_0d, Udec::new_percent(150_u64));
            assert_eq!(_1_5d * Uint128::new(2), Udec::new_percent(300_u64));
            assert_eq!(_1_5d * 2_u128, Udec::new_percent(300_u64));
            assert_eq!(_1_5d * 2_u8, Udec::new_percent(300_u64));

            let _0_75d = bt(_0d, Udec::new_percent(75_u64));
            assert_eq!(_0_75d * Uint128::new(2), Udec::new_percent(150_u64));
            assert_eq!(_0_75d * 2_u128, Udec::new_percent(150_u64));
            assert_eq!(_0_75d * 2_u8, Udec::new_percent(150_u64));
        }
    );

    dtest!( div_into_uint,
        => |_0d| {
            let _1_5d = bt(_0d, Udec::new_percent(150_u64));
            assert_eq!(_1_5d / Uint128::new(2), Udec::new_percent(75_u64));
            assert_eq!(_1_5d / 2_u128, Udec::new_percent(75_u64));
            assert_eq!(_1_5d / 2_u8, Udec::new_percent(75_u64));

            let _0_75d = bt(_0d, Udec::new_percent(75_u64));
            assert_eq!(_0_75d / Uint128::new(2), Udec::new_permille(375_u64));
            assert_eq!(_0_75d / 2_u128, Udec::new_permille(375_u64));
            assert_eq!(_0_75d / 2_u8, Udec::new_permille(375_u64));
        }
    );
}
