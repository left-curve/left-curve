use {
    crate::{
        Decimal, Fraction, Inner, MathError, MathResult, MultiplyRatio, NextNumber, Number,
        NumberConst, Sign, Uint,
    },
    bnum::types::U256,
    borsh::{BorshDeserialize, BorshSerialize},
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
    ///     grug_math::{Udec128, Uint128},
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
    U: Number,
    Uint<U>: From<u128>,
{
    /// Create a new [`Udec`] adding decimal places.
    ///
    /// ```rust
    /// use {
    ///     grug_math::{Udec128, Uint128},
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
        Self(x.into() * (Self::DECIMAL_FRACTION / 10_000).into())
    }
}

impl<U, const S: u32> Udec<U, S>
where
    Uint<U>: NumberConst + Number + From<u128>,
{
    pub fn checked_from_atomics<T>(atomics: T, decimal_places: u32) -> MathResult<Self>
    where
        T: Into<Uint<U>>,
    {
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
    pub fn checked_from_ratio<N, D>(numerator: N, denominator: D) -> MathResult<Self>
    where
        N: Into<Uint<U>>,
        D: Into<Uint<U>>,
    {
        let numerator = numerator.into();
        let denominator = denominator.into();

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
    U: NumberConst + Number,
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

    pub fn try_from_decimal<OU, const OS: u32>(other: Udec<OU, OS>) -> MathResult<Self>
    where
        Uint<U>: TryFrom<Uint<OU>>,
        MathError: From<<Uint<U> as TryFrom<Uint<OU>>>::Error>,
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
    U: Number + Copy + PartialEq,
    Uint<U>: From<u128>,
{
    fn checked_floor(self) -> MathResult<Self> {
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

    fn checked_ceil(self) -> MathResult<Self> {
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
    U: Number + Copy,
    Uint<U>: From<u128>,
{
    fn numerator(&self) -> Uint<U> {
        self.0
    }

    fn denominator() -> Uint<U> {
        Self::decimal_fraction()
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

    fn checked_add(self, other: Self) -> MathResult<Self> {
        self.0.checked_add(other.0).map(Self)
    }

    fn checked_sub(self, other: Self) -> MathResult<Self> {
        self.0.checked_sub(other.0).map(Self)
    }

    fn checked_mul(self, other: Self) -> MathResult<Self> {
        let next_result = self
            .0
            .checked_full_mul(*other.numerator())?
            .checked_div(Self::decimal_fraction().into())?;
        next_result
            .try_into()
            .map(Self)
            .map_err(|_| MathError::overflow_conversion::<_, Uint<U>>(next_result))
    }

    fn checked_div(self, other: Self) -> MathResult<Self> {
        Udec::checked_from_ratio(*self.numerator(), *other.numerator())
    }

    fn checked_rem(self, other: Self) -> MathResult<Self> {
        self.0.checked_rem(other.0).map(Self)
    }

    fn checked_pow(mut self, mut exp: u32) -> MathResult<Self> {
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

    // TODO: Check if this is the best way to implement this
    fn checked_sqrt(self) -> MathResult<Self> {
        // With the current design, U should be only unsigned number.
        // Leave this safety check here for now.
        if self.0 < Uint::ZERO {
            return Err(MathError::negative_sqrt::<Self>(self));
        }
        let hundred = Uint::TEN.checked_mul(Uint::TEN)?;
        (0..=S / 2)
            .rev()
            .find_map(|i| -> Option<MathResult<Self>> {
                let inner_mul = match hundred.checked_pow(i) {
                    Ok(val) => val,
                    Err(err) => return Some(Err(err)),
                };
                self.0.checked_mul(inner_mul).ok().map(|inner| {
                    let outer_mul = hundred.checked_pow(S / 2 - i)?;
                    Ok(Self::raw(inner.checked_sqrt()?.checked_mul(outer_mul)?))
                })
            })
            .transpose()?
            .ok_or(MathError::SqrtFailed)
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
    U: Number + Display,
    Uint<U>: Copy + From<u128>,
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
    type Err = MathError;

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
            .map_err(|_| MathError::parse_number::<Self, _, _>(input, "error parsing whole"))?
            .checked_mul(Self::decimal_fraction())
            .map_err(|_| MathError::parse_number::<Self, _, _>(input, "value too big"))?;

        if let Some(fractional_part) = parts_iter.next() {
            let fractional = fractional_part.parse::<Uint<U>>().map_err(|_| {
                MathError::parse_number::<Self, _, _>(input, "error parsing fractional")
            })?;
            let exp = (S.checked_sub(fractional_part.len() as u32)).ok_or_else(|| {
                MathError::parse_number::<Self, _, _>(
                    input,
                    format!("cannot parse more than {} fractional digits", S),
                )
            })?;

            debug_assert!(exp <= S);

            let fractional_factor = Uint::TEN.checked_pow(exp).unwrap();

            // This multiplication can't overflow because
            // fractional < 10^DECIMAL_PLACES && fractional_factor <= 10^DECIMAL_PLACES
            let fractional_part = fractional.checked_mul(fractional_factor).unwrap();

            // for negative numbers, we need to subtract the fractional part
            atomics = atomics
                .checked_add(fractional_part)
                .map_err(|_| MathError::parse_number::<Self, _, _>(input, "Value too big"))?;
        }

        if parts_iter.next().is_some() {
            return Err(MathError::parse_number::<Self, _, _>(
                input,
                "Unexpected number of dots",
            ));
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

impl<U, const S: u32> Add for Udec<U, S>
where
    U: Number + NumberConst + Copy + PartialEq + PartialOrd + Display,
    Uint<U>: NextNumber + From<u128>,
    <Uint<U> as NextNumber>::Next: Number + Copy + ToString,
{
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        self.checked_add(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl<U, const S: u32> Sub for Udec<U, S>
where
    U: Number + NumberConst + Copy + PartialEq + PartialOrd + Display,
    Uint<U>: NextNumber + From<u128>,
    <Uint<U> as NextNumber>::Next: Number + Copy + ToString,
{
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        self.checked_sub(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl<U, const S: u32> Mul for Udec<U, S>
where
    U: Number + NumberConst + Copy + PartialEq + PartialOrd + Display,
    Uint<U>: NextNumber + From<u128>,
    <Uint<U> as NextNumber>::Next: Number + Copy + ToString,
{
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        self.checked_mul(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl<U, const S: u32> Div for Udec<U, S>
where
    U: Number + NumberConst + Copy + PartialEq + PartialOrd + Display,
    Uint<U>: NextNumber + From<u128>,
    <Uint<U> as NextNumber>::Next: Number + Copy + ToString,
{
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        self.checked_div(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl<U, const S: u32> AddAssign for Udec<U, S>
where
    U: Number + NumberConst + Copy + PartialEq + PartialOrd + Display,
    Uint<U>: NextNumber + From<u128>,
    <Uint<U> as NextNumber>::Next: Number + Copy + ToString,
{
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl<U, const S: u32> SubAssign for Udec<U, S>
where
    U: Number + NumberConst + Copy + PartialEq + PartialOrd + Display,
    Uint<U>: NextNumber + From<u128>,
    <Uint<U> as NextNumber>::Next: Number + Copy + ToString,
{
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl<U, const S: u32> MulAssign for Udec<U, S>
where
    U: Number + NumberConst + Copy + PartialEq + PartialOrd + Display,
    Uint<U>: NextNumber + From<u128>,
    <Uint<U> as NextNumber>::Next: Number + Copy + ToString,
{
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl<U, const S: u32> DivAssign for Udec<U, S>
where
    U: Number + NumberConst + Copy + PartialEq + PartialOrd + Display,
    Uint<U>: NextNumber + From<u128>,
    <Uint<U> as NextNumber>::Next: Number + Copy + ToString,
{
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

// ------------------------------ concrete types -------------------------------

macro_rules! generate_decimal {
    (
        name           = $name:ident,
        inner_type     = $inner:ty,
        decimal_places = $decimal_places:expr,
        from_dec       = [$($from:ty),*],
        doc            = $doc:literal,
    ) => {
        #[doc = $doc]
        pub type $name = Udec<$inner, $decimal_places>;

        // Ex: From<U256> for Udec256
        impl From<$inner> for $name {
            fn from(value: $inner) -> Self {
                Self::raw(Uint::new(value))
            }
        }

        // Ex: From<Uint<U256>> for Udec256
        impl From<Uint<$inner>> for $name {
            fn from(value: Uint<$inner>) -> Self {
                Self::raw(value)
            }
        }

        // --- From Udec ---
        $(
            // Ex: From<Udec128> for Udec256
            impl From<$from> for $name {
                fn from(value: $from) -> Self {
                    Self::from_decimal(value)
                }
            }

            // Ex: From<Uint128> for Udec256
            impl From<Uint<<$from as Inner>::U>> for $name {
                fn from(value: Uint<<$from as Inner>::U>) -> Self {
                    Self::raw(value.into())
                }
            }

            // Ex: From<u128> for Udec256
            impl From<<$from as Inner>::U> for $name {
                fn from(value: <$from as Inner>::U) -> Self {
                    Self::raw(value.into())
                }
            }

            // Ex: TryFrom<Udec256> for Udec128
            impl TryFrom<$name> for $from {
                type Error = MathError;

                fn try_from(value: $name) -> MathResult<$from> {
                    <$from>::try_from_decimal(value)
                }
            }

            // Ex: TryFrom<Udec256> for Uint128
            impl TryFrom<$name> for Uint<<$from as Inner>::U> {
                type Error = MathError;

                fn try_from(value: $name) -> MathResult<Uint<<$from as Inner>::U>> {
                    value.0.try_into().map(Self)
                }
            }

            // Ex: TryFrom<Udec256> for u128
            impl TryFrom<$name> for <$from as Inner>::U {
                type Error = MathError;

                fn try_from(value: $name) -> MathResult<<$from as Inner>::U> {
                    value.0.try_into()
                }
            }
        )*
    };
}

generate_decimal! {
    name           = Udec128,
    inner_type     = u128,
    decimal_places = 18,
    from_dec       = [],
    doc            = "128-bit unsigned fixed-point number with 18 decimal places.",
}

generate_decimal! {
    name           = Udec256,
    inner_type     = U256,
    decimal_places = 18,
    from_dec       = [Udec128],
    doc            = "256-bit unsigned fixed-point number with 18 decimal places.",
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        crate::{Number, Udec128, Udec256},
        bnum::{
            errors::TryFromIntError,
            types::{U256, U512},
        },
        std::str::FromStr,
    };

    #[test]
    fn t1() {
        assert_eq!(Udec128::one() + Udec128::one(), Udec128::new(2_u128));

        assert_eq!(
            Udec128::new(10_u128)
                .checked_add(Udec128::new(20_u128))
                .unwrap(),
            Udec128::new(30_u128)
        );

        assert_eq!(
            Udec128::new(3_u128)
                .checked_rem(Udec128::new(2_u128))
                .unwrap(),
            Udec128::from_str("1").unwrap()
        );

        assert_eq!(
            Udec128::from_str("3.5")
                .unwrap()
                .checked_rem(Udec128::new(2_u128))
                .unwrap(),
            Udec128::from_str("1.5").unwrap()
        );

        assert_eq!(
            Udec128::from_str("3.5")
                .unwrap()
                .checked_rem(Udec128::from_str("2.7").unwrap())
                .unwrap(),
            Udec128::from_str("0.8").unwrap()
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
        let foo = Udec128::new(10_u128);
        assert_eq!(Udec256::new(10_u128), Udec256::from(foo));

        let foo = Udec256::new(10_u128);
        assert_eq!(Udec128::new(10_u128), Udec128::try_from(foo).unwrap())
    }
}
