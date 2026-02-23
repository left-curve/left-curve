//! A Rust type system that encapsulates the dimensionality of values.

use {
    grug::{
        Dec128_6, Duration, Exponentiate, Int128, IsZero, MathResult, Number as _, NumberConst,
        Sign, Signed, Uint128, Unsigned,
    },
    std::{
        fmt,
        marker::PhantomData,
        ops::{Neg, Sub},
    },
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
    ) -> MathResult<
        Number<
            <(Q, Q1) as TypeAdd>::Output,
            <(U, U1) as TypeAdd>::Output,
            <(D, D1) as TypeAdd>::Output,
        >,
    >
    where
        (Q, Q1): TypeAdd,
        (U, U1): TypeAdd,
        (D, D1): TypeAdd,
    {
        self.inner.checked_mul(rhs.inner).map(Number::new)
    }

    pub fn checked_div<Q1, U1, D1>(
        self,
        rhs: Number<Q1, U1, D1>,
    ) -> MathResult<
        Number<
            <(Q, Q1) as TypeSub>::Output,
            <(U, U1) as TypeSub>::Output,
            <(D, D1) as TypeSub>::Output,
        >,
    >
    where
        (Q, Q1): TypeSub,
        (U, U1): TypeSub,
        (D, D1): TypeSub,
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
pub type Dimensionless = Number<Zero, Zero, Zero>;

/// A duration of time, as number of days.
pub type Days = Number<Zero, Zero, Succ>;

impl Days {
    pub fn from_duration(duration: Duration) -> MathResult<Self> {
        const NANOS_PER_DAY: i128 = 24 * 60 * 60 * 1_000_000_000;

        let nanos = duration.into_nanos();
        let days = Dec128_6::checked_from_ratio(nanos as i128, NANOS_PER_DAY)?;

        Ok(Self::new(days))
    }
}

/// Quantity of an asset, in _human unit_: quantity¹
pub type Quantity = Number<Succ, Zero, Zero>;

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
pub type UsdValue = Number<Zero, Succ, Zero>;

/// Price of an asset: usd¹⋅quantity⁻¹
pub type UsdPrice = Number<Pred, Succ, Zero>;

/// Cumulative funding accrued per unit of position size: usd¹⋅quantity⁻¹
///
/// Dimensionally identical to `UsdPrice` but represents a distinct concept:
/// the running accumulator used to compute a position's funding payment.
pub type FundingPerUnit = Number<Pred, Succ, Zero>;

/// Funding rate: duration⁻¹
pub type FundingRate = Number<Zero, Zero, Pred>;

/// Funding velocity, i.e. the rate at which funding rate changes: duration⁻²
pub type FundingVelocity = Number<Zero, Zero, Pred<Pred>>;

// ------------------------ Arithmetic types and traits ------------------------

/// Represents zero at the type level.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Zero;

/// Represents the sucessor of a type.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Succ<T = Zero>(PhantomData<T>);

/// Represents the predecessor of a type.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pred<T = Zero>(PhantomData<T>);

/// Describes when two values are multiplied, how their types should be added.
pub trait TypeAdd {
    type Output;
}

impl TypeAdd for (Zero, Zero) {
    type Output = Zero;
}

impl<T> TypeAdd for (Zero, Succ<T>) {
    type Output = Succ<T>;
}

impl<T> TypeAdd for (Zero, Pred<T>) {
    type Output = Pred<T>;
}

impl<T> TypeAdd for (Succ<T>, Zero) {
    type Output = Succ<T>;
}

impl<T> TypeAdd for (Pred<T>, Zero) {
    type Output = Pred<T>;
}

// Concrete TypeAdd impls for the Peano numbers used in the system.
// These replace recursive blanket impls to avoid trait-resolver overflow.

// Succ<Zero> + Pred<Zero> = Zero  (1 + -1 = 0)
impl TypeAdd for (Succ<Zero>, Pred<Zero>) {
    type Output = Zero;
}

// Pred<Zero> + Succ<Zero> = Zero  (-1 + 1 = 0)
impl TypeAdd for (Pred<Zero>, Succ<Zero>) {
    type Output = Zero;
}

// Pred<Pred<Zero>> + Succ<Zero> = Pred<Zero>  (-2 + 1 = -1)
impl TypeAdd for (Pred<Pred<Zero>>, Succ<Zero>) {
    type Output = Pred<Zero>;
}

// Pred<Zero> + Pred<Zero> = Pred<Pred<Zero>>  (-1 + -1 = -2)
impl TypeAdd for (Pred<Zero>, Pred<Zero>) {
    type Output = Pred<Pred<Zero>>;
}

/// Describes when two values are divided, how their types should be subtracted.
pub trait TypeSub {
    type Output;
}

impl TypeSub for (Zero, Zero) {
    type Output = Zero;
}

impl<T> TypeSub for (Succ<T>, Zero) {
    type Output = Succ<T>;
}

impl<T> TypeSub for (Pred<T>, Zero) {
    type Output = Pred<T>;
}

// Concrete TypeSub impls for the Peano numbers used in the system.
// These replace recursive blanket impls to avoid trait-resolver overflow.

// Zero - Pred<Zero> = Succ<Zero>  (0 - (-1) = 1)
impl TypeSub for (Zero, Pred<Zero>) {
    type Output = Succ<Zero>;
}

// Succ<Zero> - Succ<Zero> = Zero  (1 - 1 = 0)
impl TypeSub for (Succ<Zero>, Succ<Zero>) {
    type Output = Zero;
}
