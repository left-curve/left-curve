use {
    crate::{
        Decimal, FixedPoint, Fraction, Inner, IsZero, MathError, MathResult, MultiplyRatio,
        NextNumber, Number, NumberConst, Sign, Uint, Uint128, Uint256,
    },
    bnum::types::U256,
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{de, ser},
    std::{
        cmp::Ordering,
        fmt::{self, Display, Write},
        marker::PhantomData,
        ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Rem, RemAssign, Sub, SubAssign},
        str::FromStr,
    },
};

// ------------------------------- generic type --------------------------------

#[derive(
    BorshSerialize, BorshDeserialize, Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord,
)]
pub struct Udec<U>(pub(crate) Uint<U>);

impl<U> Udec<U> {
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

impl<U> Udec<U>
where
    Self: FixedPoint<U>,
    Uint<U>: NumberConst + Number,
{
    pub fn checked_from_atomics<T>(atomics: T, decimal_places: u32) -> MathResult<Self>
    where
        T: Into<Uint<U>>,
    {
        let atomics = atomics.into();

        let inner = match decimal_places.cmp(&Self::DECIMAL_PLACES) {
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

impl<U> Udec<U>
where
    Self: FixedPoint<U>,
    Uint<U>: MultiplyRatio,
    Uint<U>: MultiplyRatio,
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
// different word size.
//
// We can't implement the `From` and `TryFrom` traits here, because it would
// conflict with the standard library's `impl From<T> for T`, as we can't yet
// specify that `U != OU` with stable Rust.
impl<U> Udec<U>
where
    U: NumberConst + Number,
{
    pub fn from_decimal<OU>(other: Udec<OU>) -> Self
    where
        Uint<U>: From<Uint<OU>>,
    {
        Self(Uint::<U>::from(other.0))
    }

    pub fn try_from_decimal<OU>(other: Udec<OU>) -> MathResult<Self>
    where
        Uint<U>: TryFrom<Uint<OU>>,
        MathError: From<<Uint<U> as TryFrom<Uint<OU>>>::Error>,
    {
        Ok(Uint::<U>::try_from(other.0).map(Self)?)
    }
}

impl<U> Decimal for Udec<U>
where
    Self: FixedPoint<U>,
    U: Number + Copy + PartialEq,
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
        Ok(Self(self.0 - self.0.checked_rem(Self::DECIMAL_FRACTION)?))
    }

    fn checked_ceil(self) -> MathResult<Self> {
        let floor = self.checked_floor()?;
        if floor == self {
            Ok(floor)
        } else {
            floor.0.checked_add(Self::DECIMAL_FRACTION).map(Self)
        }
    }
}

impl<U> Inner for Udec<U> {
    type U = U;
}

impl<U> Sign for Udec<U> {
    fn abs(self) -> Self {
        self
    }

    fn is_negative(&self) -> bool {
        false
    }
}

impl<U> Fraction<U> for Udec<U>
where
    Self: FixedPoint<U>,
    U: Number + IsZero + Display + Copy,
    Uint<U>: MultiplyRatio,
{
    fn numerator(&self) -> Uint<U> {
        self.0
    }

    fn denominator() -> Uint<U> {
        Self::DECIMAL_FRACTION
    }

    fn inv(&self) -> MathResult<Self> {
        if self.is_zero() {
            Err(MathError::division_by_zero(self))
        } else {
            Self::checked_from_ratio(Self::DECIMAL_FRACTION, self.0)
        }
    }
}

impl<U> IsZero for Udec<U>
where
    U: IsZero,
{
    fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
}

impl<U> Number for Udec<U>
where
    Self: FixedPoint<U> + NumberConst,
    U: NumberConst + Number + IsZero + Copy + PartialEq + PartialOrd + Display,
    Uint<U>: NextNumber + IsZero + Display,
    <Uint<U> as NextNumber>::Next: Number + IsZero + Copy + ToString,
{
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
            .checked_div(Self::DECIMAL_FRACTION.into())?;
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
            return Ok(Self::ONE);
        }

        let mut y = Udec::ONE;
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
        (0..=Self::DECIMAL_PLACES / 2)
            .rev()
            .find_map(|i| -> Option<MathResult<Self>> {
                let inner_mul = match hundred.checked_pow(i) {
                    Ok(val) => val,
                    Err(err) => return Some(Err(err)),
                };
                self.0.checked_mul(inner_mul).ok().map(|inner| {
                    let outer_mul = Uint::TEN.checked_pow(Self::DECIMAL_PLACES / 2 - i)?;
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

impl<U> Display for Udec<U>
where
    Self: FixedPoint<U>,
    U: Number + IsZero + Display,
    Uint<U>: Copy,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let decimals = Self::DECIMAL_FRACTION;
        let whole = (self.0) / decimals;
        let fractional = (self.0).checked_rem(decimals).unwrap();

        if fractional.is_zero() {
            write!(f, "{whole}")?;
        } else {
            let fractional_string = format!(
                "{:0>padding$}",
                fractional.0,
                padding = Self::DECIMAL_PLACES as usize
            );
            f.write_str(&whole.to_string())?;
            f.write_char('.')?;
            f.write_str(&fractional_string.trim_end_matches('0').replace('-', ""))?;
        }

        Ok(())
    }
}

impl<U> FromStr for Udec<U>
where
    Self: FixedPoint<U>,
    Uint<U>: NumberConst + Number + Display + FromStr,
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
            .checked_mul(Self::DECIMAL_FRACTION)
            .map_err(|_| MathError::parse_number::<Self, _, _>(input, "value too big"))?;

        if let Some(fractional_part) = parts_iter.next() {
            let fractional = fractional_part.parse::<Uint<U>>().map_err(|_| {
                MathError::parse_number::<Self, _, _>(input, "error parsing fractional")
            })?;
            let exp = (Self::DECIMAL_PLACES.checked_sub(fractional_part.len() as u32)).ok_or_else(
                || {
                    MathError::parse_number::<Self, _, _>(
                        input,
                        format!(
                            "cannot parse more than {} fractional digits",
                            Self::DECIMAL_FRACTION
                        ),
                    )
                },
            )?;

            debug_assert!(exp <= Self::DECIMAL_PLACES);

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

impl<U> ser::Serialize for Udec<U>
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

impl<'de, U> de::Deserialize<'de> for Udec<U>
where
    Udec<U>: FromStr,
    <Udec<U> as FromStr>::Err: Display,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_str(DecimalVisitor::new())
    }
}

struct DecimalVisitor<U> {
    _marker: PhantomData<U>,
}

impl<U> DecimalVisitor<U> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<'de, U> de::Visitor<'de> for DecimalVisitor<U>
where
    Udec<U>: FromStr,
    <Udec<U> as FromStr>::Err: Display,
{
    type Value = Udec<U>;

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

impl<U> Add for Udec<U>
where
    Self: Number,
{
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        self.checked_add(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl<U> Sub for Udec<U>
where
    Self: Number,
{
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        self.checked_sub(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl<U> Mul for Udec<U>
where
    Self: Number,
{
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        self.checked_mul(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl<U> Div for Udec<U>
where
    Self: Number,
{
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        self.checked_div(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl<U> Rem for Udec<U>
where
    Self: Number,
{
    type Output = Self;

    fn rem(self, rhs: Self) -> Self::Output {
        self.checked_rem(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl<U> AddAssign for Udec<U>
where
    Self: Number + Copy,
    Self: Number + Copy,
{
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl<U> SubAssign for Udec<U>
where
    Self: Number + Copy,
    Self: Number + Copy,
{
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl<U> MulAssign for Udec<U>
where
    Self: Number + Copy,
    Self: Number + Copy,
{
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl<U> DivAssign for Udec<U>
where
    Self: Number + Copy,
    Self: Number + Copy,
{
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

impl<U> RemAssign for Udec<U>
where
    Self: Number + Copy,
{
    fn rem_assign(&mut self, rhs: Self) {
        *self = *self % rhs;
    }
}

impl<IntoUintU, U> Mul<IntoUintU> for Udec<U>
where
    U: Number,
    IntoUintU: Into<Uint<U>>,
{
    type Output = Self;

    fn mul(self, rhs: IntoUintU) -> Self::Output {
        Self::raw(self.0 * rhs.into())
    }
}

impl<IntoUintU, U> Div<IntoUintU> for Udec<U>
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

macro_rules! generate_decimal {
    (
        name              = $name:ident,
        inner_type        = $inner:ty,
        inner_constructor = $constructor:expr,
        from_dec          = [$($from:ty),*],
        doc               = $doc:literal,
    ) => {
        #[doc = $doc]
        pub type $name = Udec<$inner>;

        impl NumberConst for $name {
            const MAX: Self = Self(Uint::MAX);
            const MIN: Self = Self(Uint::MIN);
            const ONE: Self = Self(Self::DECIMAL_FRACTION);
            const TEN: Self = Self($constructor(10_u128.pow(Self::DECIMAL_PLACES + 1)));
            const ZERO: Self = Self(Uint::ZERO);
        }

        impl $name {
            /// Create a new [`Udec`] adding decimal places.
            ///
            /// ```rust
            /// use {
            ///     grug_math::{Udec128, Uint128},
            ///     std::str::FromStr,
            /// };
            ///
            /// let decimal = Udec128::new(100);
            /// assert_eq!(decimal, Udec128::from_str("100.0").unwrap());
            /// ```
            pub const fn new(x: u128) -> Self {
                Self($constructor(x * 10_u128.pow(Self::DECIMAL_PLACES)))
            }
            pub const fn new_percent(x: u128) -> Self {
                Self($constructor(x * 10_u128.pow(Self::DECIMAL_PLACES - 2)))
            }

            pub const fn new_permille(x: u128) -> Self {
                Self($constructor(x * 10_u128.pow(Self::DECIMAL_PLACES - 3)))
            }

            pub const fn new_bps(x: u128) -> Self {
                Self($constructor(x * 10_u128.pow(Self::DECIMAL_PLACES - 4)))
            }
        }

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
    name              = Udec128,
    inner_type        = u128,
    inner_constructor = Uint128::new,
    from_dec          = [],
    doc               = "128-bit unsigned fixed-point number with 18 decimal places.",
}

generate_decimal! {
    name              = Udec256,
    inner_type        = U256,
    inner_constructor = Uint256::new_from_u128,
    from_dec          = [Udec128],
    doc               = "256-bit unsigned fixed-point number with 18 decimal places.",
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        crate::{Number, NumberConst, Udec128, Udec256},
        bnum::{
            errors::TryFromIntError,
            types::{U256, U512},
        },
        std::str::FromStr,
    };

    #[test]
    fn t1() {
        assert_eq!(Udec128::ONE + Udec128::ONE, Udec128::new(2_u128));

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
