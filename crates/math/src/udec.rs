use {
    crate::{
        FixedPoint, Inner, Int128, Int256, IsZero, MathError, MathResult, MultiplyRatio, Number,
        NumberConst, Sign, Uint, Uint128, Uint256,
    },
    bnum::types::{I256, U256},
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{de, ser},
    std::{
        cmp::Ordering,
        fmt::{self, Display, Write},
        marker::PhantomData,
        ops::{
            Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Sub, SubAssign,
        },
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
    Self: FixedPoint<U>,
    U: NumberConst + Number,
{
    pub fn from_decimal<OU>(other: Udec<OU>) -> Self
    where
        Uint<U>: From<Uint<OU>>,
        Udec<OU>: FixedPoint<OU>,
    {
        match Udec::<OU>::DECIMAL_PLACES.cmp(&Udec::<U>::DECIMAL_PLACES) {
            Ordering::Greater => {
                // There are not overflow problem for adjusted_precision
                let adjusted_precision = Uint::<U>::TEN
                    .checked_pow(Udec::<OU>::DECIMAL_PLACES - Udec::<U>::DECIMAL_PLACES)
                    .unwrap();
                Self(Uint::<U>::from(other.0) / adjusted_precision)
            },
            Ordering::Less => {
                let adjusted_precision = Uint::<U>::TEN
                    .checked_pow(Udec::<U>::DECIMAL_PLACES - Udec::<OU>::DECIMAL_PLACES)
                    .unwrap();
                Self(Uint::<U>::from(other.0) * adjusted_precision)
            },
            Ordering::Equal => Self(Uint::<U>::from(other.0)),
        }
    }

    pub fn try_from_decimal<OU>(other: Udec<OU>) -> MathResult<Self>
    where
        Uint<U>: TryFrom<Uint<OU>, Error = MathError>,
        Udec<OU>: FixedPoint<OU>,
        Uint<OU>: Number + NumberConst,
    {
        match Udec::<OU>::DECIMAL_PLACES.cmp(&Udec::<U>::DECIMAL_PLACES) {
            Ordering::Greater => {
                // adjusted precision in Uint<OU> prevent overflow in the pow
                let adjusted_precision = Uint::<OU>::TEN
                    .checked_pow(Udec::<OU>::DECIMAL_PLACES - Udec::<U>::DECIMAL_PLACES)?;
                other
                    .0
                    .checked_div(adjusted_precision)
                    .map(Uint::<U>::try_from)?
                    .map(Self)
            },
            Ordering::Less => {
                // adjusted precision in Uint<U> prevent overflow in the pow
                let adjusted_precision = Uint::<U>::TEN
                    .checked_pow(Udec::<U>::DECIMAL_PLACES - Udec::<OU>::DECIMAL_PLACES)?;
                Uint::<U>::try_from(other.0)?
                    .checked_mul(adjusted_precision)
                    .map(Self)
            },
            Ordering::Equal => Uint::<U>::try_from(other.0).map(Self),
        }
    }
}

impl<U> Neg for Udec<U>
where
    U: Neg<Output = U>,
{
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

impl<U> Display for Udec<U>
where
    Self: FixedPoint<U>,
    U: Number + IsZero + Display,
    Uint<U>: Copy + Sign,
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
                fractional.abs().0,
                padding = Self::DECIMAL_PLACES as usize
            );
            if whole.is_negative() || fractional.is_negative() {
                f.write_char('-')?;
            }
            f.write_str(&whole.abs().to_string())?;
            f.write_char('.')?;
            f.write_str(fractional_string.trim_end_matches('0'))?;
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
            // We can't check if atomics is negative because -0 is positive
            atomics = if input.starts_with("-") {
                atomics.checked_sub(fractional_part)
            } else {
                atomics.checked_add(fractional_part)
            }
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
        deserializer.deserialize_str(UdecVisitor::new())
    }
}

struct UdecVisitor<U> {
    _marker: PhantomData<U>,
}

impl<U> UdecVisitor<U> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<'de, U> de::Visitor<'de> for UdecVisitor<U>
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
{
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl<U> SubAssign for Udec<U>
where
    Self: Number + Copy,
{
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl<U> MulAssign for Udec<U>
where
    Self: Number + Copy,
{
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl<U> DivAssign for Udec<U>
where
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

// ------------------------------ concrete types -------------------------------

macro_rules! generate_decimal {
    (
        name              = $name:ident,
        inner_type        = $inner:ty,
        inner_constructor = $constructor:expr,
        base_constructor  = $base_constructor:ty,
        from_dec          = [$($from:ty),*],
        doc               = $doc:literal,
    ) => {
        paste::paste! {
            #[doc = $doc]
            pub type $name = Udec<$inner>;
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
                pub const fn new(x: $base_constructor) -> Self {
                    Self($constructor(x * [<10_$base_constructor>].pow(Self::DECIMAL_PLACES)))
                }
                pub const fn new_percent(x: $base_constructor) -> Self {
                    Self($constructor(x * [<10_$base_constructor>].pow(Self::DECIMAL_PLACES - 2)))
                }

                pub const fn new_permille(x: $base_constructor) -> Self {
                    Self($constructor(x * [<10_$base_constructor>].pow(Self::DECIMAL_PLACES - 3)))
                }

                pub const fn new_bps(x: $base_constructor) -> Self {
                    Self($constructor(x * [<10_$base_constructor>].pow(Self::DECIMAL_PLACES - 4)))
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
        }
    };
    (
        type              = Signed,
        name              = $name:ident,
        inner_type        = $inner:ty,
        inner_constructor = $constructor:expr,
        from_dec          = [$($from:ty),*],
        doc               = $doc:literal,
    ) => {
        generate_decimal! {
            name              = $name,
            inner_type        = $inner,
            inner_constructor = $constructor,
            base_constructor  = i128,
            from_dec          = [$($from),*],
            doc               = $doc,
        }
    };
    (
        type              = Unsigned,
        name              = $name:ident,
        inner_type        = $inner:ty,
        inner_constructor = $constructor:expr,
        from_dec          = [$($from:ty),*],
        doc               = $doc:literal,
    ) => {
        generate_decimal! {
            name              = $name,
            inner_type        = $inner,
            inner_constructor = $constructor,
            base_constructor  = u128,
            from_dec          = [$($from),*],
            doc               = $doc,
        }
    }
}

generate_decimal! {
    type              = Unsigned,
    name              = Udec128,
    inner_type        = u128,
    inner_constructor = Uint128::new,
    from_dec          = [],
    doc               = "128-bit unsigned fixed-point number with 18 decimal places.",
}

generate_decimal! {
    type              = Unsigned,
    name              = Udec256,
    inner_type        = U256,
    inner_constructor = Uint256::new_from_u128,
    from_dec          = [Udec128],
    doc               = "256-bit unsigned fixed-point number with 18 decimal places.",
}

generate_decimal! {
    type              = Signed,
    name              = Dec128,
    inner_type        = i128,
    inner_constructor = Int128::new,
    from_dec          = [],
    doc               = "128-bit signed fixed-point number with 18 decimal places.",
}

generate_decimal! {
    type              = Signed,
    name              = Dec256,
    inner_type        = I256,
    inner_constructor = Int256::new_from_i128,
    from_dec          = [],
    doc               = "256-bit signed fixed-point number with 18 decimal places.",
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        crate::{Dec128, Number, NumberConst, Udec128, Udec256, Uint64},
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

    #[test]
    fn neg_to_string_works() {
        assert_eq!(Dec128::new(-1).to_string(), "-1");
        assert_eq!(Dec128::new_percent(-10).to_string(), "-0.1");
        assert_eq!(Dec128::new_percent(-110).to_string(), "-1.1");
        assert_eq!(Dec128::new(1).to_string(), "1");
        assert_eq!(Dec128::new_percent(10).to_string(), "0.1");
        assert_eq!(Dec128::new_percent(110).to_string(), "1.1");
    }

    #[test]
    fn new_from_str_works() {
        assert_eq!(Dec128::from_str("0.5").unwrap(), Dec128::new_percent(50));
        assert_eq!(Dec128::from_str("1").unwrap(), Dec128::new(1));
        assert_eq!(Dec128::from_str("1.05").unwrap(), Dec128::new_percent(105));
        assert_eq!(Dec128::from_str("-0.5").unwrap(), Dec128::new_percent(-50));
        assert_eq!(Dec128::from_str("-1").unwrap(), Dec128::new(-1));
        assert_eq!(
            Dec128::from_str("-1.05").unwrap(),
            Dec128::new_percent(-105)
        );
    }

    #[test]
    fn neg_works() {
        assert_eq!(-Dec128::new_percent(-105), Dec128::new_percent(105));
        assert_eq!(-Dec128::new_percent(50), Dec128::new_percent(-50));
    }

    #[test]
    fn different_places_conversion() {
        use super::*;

        generate_decimal! {
            name              = Udec64_12,
            inner_type        = u64,
            inner_constructor = Uint64::new,
            base_constructor  = u64,
            from_dec          = [],
            doc               = "128-bit unsigned fixed-point number with 18 decimal places.",
        }

        impl FixedPoint<u64> for Udec64_12 {
            const DECIMAL_FRACTION: crate::Uint<u64> =
                crate::Uint64::new(10_u64.pow(Self::DECIMAL_PLACES));
            const DECIMAL_PLACES: u32 = 12;
        }

        let d64 = Udec64_12::from_str("1.123456789012").unwrap();
        let d128 = Udec128::from_decimal(d64);
        assert_eq!(d64.to_string(), d128.to_string());

        let d128 = Udec128::from_str("1.123456789012").unwrap();
        let d64 = Udec64_12::try_from_decimal(d128).unwrap();
        assert_eq!(d64.to_string(), d128.to_string());

        let d128 = Udec128::from_str("1.123456789012345678").unwrap();
        let d64 = Udec64_12::try_from_decimal(d128).unwrap();
        assert_eq!(d64.to_string(), "1.123456789012");

        let d128 = Udec128::raw((u64::MAX).into());
        let d64 = Udec64_12::try_from_decimal(d128).unwrap();
        assert_eq!(d64.to_string(), "18.446744073709");

        let d128 = Udec128::from_str("18446744.073709551615").unwrap();
        let d64 = Udec64_12::try_from_decimal(d128).unwrap();
        assert_eq!(d64.to_string(), "18446744.073709551615");

        let d128 = Udec128::from_str("18446744.073709551616").unwrap();
        assert!(matches!(
            Udec64_12::try_from_decimal(d128),
            Err(MathError::OverflowConversion { .. })
        ));
    }
}
