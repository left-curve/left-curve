//! A Rust type system that encapsulates the dimensionality of values.

use {
    grug::{
        Dec128_6, Duration, Exponentiate, Int128, IsZero, MathResult, Number as _, NumberConst,
        Sign, Signed, Uint128, Unsigned,
    },
    std::{
        fmt,
        marker::PhantomData,
        ops::{Add, Neg, Sub},
    },
    typenum::{N1, N2, P1, Z0},
};

// -------------------------------- Number type --------------------------------

/// A signed, fixed-point decimal number with three dimensions typically used in
/// financial settings: quantity, USD value, and time duration.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Number<Q, U, D> {
    inner: Dec128_6,
    _quantity: PhantomData<Q>,
    _usd: PhantomData<U>,
    _duration: PhantomData<D>,
}

impl<Q, U, D> Number<Q, U, D> {
    pub const HALF: Self = Self::new(Dec128_6::raw(Int128::new(500_000)));
    pub const ONE: Self = Self::new(Dec128_6::ONE);
    pub const ZERO: Self = Self::new(Dec128_6::ZERO);

    pub const fn new(inner: Dec128_6) -> Self {
        Self {
            inner,
            _quantity: PhantomData,
            _usd: PhantomData,
            _duration: PhantomData,
        }
    }

    pub const fn new_int(int: i128) -> Self {
        Self::new(Dec128_6::new(int))
    }

    pub const fn new_raw(raw: i128) -> Self {
        Self::new(Dec128_6::raw(Int128::new(raw)))
    }

    pub const fn new_permille(n: i128) -> Self {
        Self::new(Dec128_6::new_permille(n))
    }

    pub fn is_non_zero(&self) -> bool {
        self.inner.is_non_zero()
    }

    pub fn is_positive(&self) -> bool {
        self.inner.is_positive()
    }

    pub fn is_negative(&self) -> bool {
        self.inner.is_negative()
    }

    pub fn into_inner(self) -> Dec128_6 {
        self.inner
    }

    pub fn checked_abs(self) -> MathResult<Self> {
        self.inner.checked_abs().map(Self::new)
    }

    pub fn checked_add(self, rhs: Self) -> MathResult<Self> {
        self.inner.checked_add(rhs.inner).map(Self::new)
    }

    pub fn checked_sub(self, rhs: Self) -> MathResult<Self> {
        self.inner.checked_sub(rhs.inner).map(Self::new)
    }

    pub fn checked_mul<Q1, U1, D1>(
        self,
        rhs: Number<Q1, U1, D1>,
    ) -> MathResult<Number<<Q as Add<Q1>>::Output, <U as Add<U1>>::Output, <D as Add<D1>>::Output>>
    where
        Q: Add<Q1>,
        U: Add<U1>,
        D: Add<D1>,
    {
        self.inner.checked_mul(rhs.inner).map(Number::new)
    }

    pub fn checked_div<Q1, U1, D1>(
        self,
        rhs: Number<Q1, U1, D1>,
    ) -> MathResult<Number<<Q as Sub<Q1>>::Output, <U as Sub<U1>>::Output, <D as Sub<D1>>::Output>>
    where
        Q: Sub<Q1>,
        U: Sub<U1>,
        D: Sub<D1>,
    {
        self.inner.checked_div(rhs.inner).map(Number::new)
    }
}

impl<Q, U, D> Number<Q, U, D>
where
    Q: Copy,
    U: Copy,
    D: Copy,
{
    pub fn checked_add_assign(&mut self, rhs: Self) -> MathResult<()> {
        *self = self.checked_add(rhs)?;
        Ok(())
    }

    pub fn checked_sub_assign(&mut self, rhs: Self) -> MathResult<()> {
        *self = self.checked_sub(rhs)?;
        Ok(())
    }
}

impl<Q, U, D> Number<Q, U, D>
where
    Q: Ord,
    U: Ord,
    D: Ord,
{
    pub fn clamp(self, min: Self, max: Self) -> Self {
        self.min(max).max(min)
    }
}

impl<Q, U, D> Neg for Number<Q, U, D> {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.inner) // Panics when the inner value is `i128::MIN`.
    }
}

impl<Q, U, D> Sub for Number<Q, U, D> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.inner - rhs.inner)
    }
}

impl<Q, U, D> fmt::Display for Number<Q, U, D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl<Q, U, D> serde::ser::Serialize for Number<Q, U, D> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.inner.serialize(serializer)
    }
}

impl<'de, Q, U, D> serde::de::Deserialize<'de> for Number<Q, U, D> {
    fn deserialize<DS>(deserializer: DS) -> Result<Self, DS::Error>
    where
        DS: serde::Deserializer<'de>,
    {
        serde::de::Deserialize::deserialize(deserializer).map(Self::new)
    }
}

impl<Q, U, D> borsh::BorshSerialize for Number<Q, U, D> {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        borsh::BorshSerialize::serialize(&self.inner, writer)
    }
}

impl<Q, U, D> borsh::BorshDeserialize for Number<Q, U, D> {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        borsh::BorshDeserialize::deserialize_reader(reader).map(Self::new)
    }
}

// ---------------------------------- Aliases ----------------------------------

/// A dimensionless scalar (pure number, no physical units).
pub type Dimensionless = Number<Z0, Z0, Z0>;

/// A duration of time, as number of days.
pub type Days = Number<Z0, Z0, P1>;

impl Days {
    pub fn from_duration(duration: Duration) -> MathResult<Self> {
        const NANOS_PER_DAY: i128 = 24 * 60 * 60 * 1_000_000_000;

        let nanos = duration.into_nanos();
        let days = Dec128_6::checked_from_ratio(nanos as i128, NANOS_PER_DAY)?;

        Ok(Self::new(days))
    }
}

/// Quantity of an asset, in _human unit_: quantity¹
pub type Quantity = Number<P1, Z0, Z0>;

impl Quantity {
    /// Convert an asset amount from base unit (represented by the `Uint128` type)
    /// to human unit (represented by the `Number` type).
    pub fn from_base(base_amount: Uint128, decimals: u32) -> MathResult<Self> {
        base_amount
            .checked_into_dec()?
            .checked_into_signed()?
            .checked_div(Dec128_6::TEN.checked_pow(decimals)?)
            .map(Self::new)
    }

    /// Convert an asset amount from human unit (represented by the `Number` type)
    /// to base unit (represented by the `Uint128` type).
    /// Floor the number when rounding to integer.
    pub fn into_base_floor(self, decimals: u32) -> MathResult<Uint128> {
        self.inner
            .checked_mul(Dec128_6::TEN.checked_pow(decimals)?)?
            .checked_into_unsigned()
            .map(|dec| dec.into_int_floor())
    }

    /// Convert an asset amount from human unit (represented by the `Number` type)
    /// to base unit (represented by the `Uint128` type).
    /// Ceil the number when rounding to integer.
    pub fn into_base_ceil(self, decimals: u32) -> MathResult<Uint128> {
        self.inner
            .checked_mul(Dec128_6::TEN.checked_pow(decimals)?)?
            .checked_into_unsigned()
            .map(|dec| dec.into_int_ceil())
    }
}

/// Amount of US dollars: usd¹
pub type UsdValue = Number<Z0, P1, Z0>;

/// Price of an asset: usd¹⋅quantity⁻¹
pub type UsdPrice = Number<N1, P1, Z0>;

/// Cumulative funding accrued per unit of position size: usd¹⋅quantity⁻¹
///
/// Dimensionally identical to `UsdPrice` but represents a distinct concept:
/// the running accumulator used to compute a position's funding payment.
pub type FundingPerUnit = Number<N1, P1, Z0>;

/// Funding rate: duration⁻¹
pub type FundingRate = Number<Z0, Z0, N1>;

/// Funding velocity, i.e. the rate at which funding rate changes: duration⁻²
pub type FundingVelocity = Number<Z0, Z0, N2>;
