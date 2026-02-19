//! In smart contracts, when we come across a number, let's say 123.456, it's
//! sometimes confusing what it means -- does it mean the USD value ("notional value")
//! of an asset, or the quantity of an asset? If it's a quantity, is it in the
//! human unit (e.g. BTC), or the base unit (satoshi or 1e-8 BTC)?
//! To avoid the confusion, we define a Rust type for each of these types of number.

use {
    grug::{Dec128_6, Int128, MathResult, Number, NumberConst, Sign, Uint128},
    std::{
        marker::PhantomData,
        ops::{Neg, Sub},
    },
};

// TODO: merge this into the `grug::Inner` trait
pub trait FromInner {
    type Inner;

    fn from_inner(inner: Self::Inner) -> Self;
}

// -------------------------------- Base amount --------------------------------

/// A quantity of asset in _base unit_. E.g. a value of 1234, in the context of
/// USDC, which has 6 decimal places, means 1234 uusdc or 0.001234 USDC.
///
/// In Dango, this is used to denote the quantity of the settlement currency for
/// perpetual futures contracts, and tokenized shares in the counterparty vault.
#[grug::derive(Serde, Borsh)]
#[derive(Copy)]
pub struct BaseAmount(Uint128);

// ------------------------------- Human amount --------------------------------

/// A quantity of asset in _human unit_. E.g. a value of 1.234, in the context of
/// BTC, means 1.234 BTC (not BTC's base unit, which is satoshi or 1e-8 BTC).
///
/// In Dango, this is used to denote the quantity of perpetual futures contracts.
/// E.g. +1.234 units of the BTCUSD-PERP contract indicates a long exposure to
/// 1.234 BTC.
///
/// This value can be negative, in which case it represents a short position.
#[grug::derive(Serde, Borsh)]
#[derive(Copy, PartialOrd, Ord)]
pub struct HumanAmount(Dec128_6);

impl HumanAmount {
    pub const ZERO: Self = Self(Dec128_6::ZERO);

    pub fn is_positive(self) -> bool {
        self.0.is_positive()
    }

    pub fn is_negative(self) -> bool {
        self.0.is_negative()
    }

    pub fn checked_add(self, rhs: Self) -> MathResult<Self> {
        let inner = self.0.checked_add(rhs.0)?;
        Ok(Self(inner))
    }

    pub fn checked_mul<N>(self, ratio: Ratio<N, Self>) -> MathResult<N>
    where
        N: FromInner<Inner = Dec128_6>,
    {
        let inner = self.0.checked_mul(ratio.inner)?;
        Ok(N::from_inner(inner))
    }

    pub fn checked_div<D>(self, ratio: Ratio<Self, D>) -> MathResult<D>
    where
        D: FromInner<Inner = Dec128_6>,
    {
        let inner = self.0.checked_div(ratio.inner)?;
        Ok(D::from_inner(inner))
    }
}

impl FromInner for HumanAmount {
    type Inner = Dec128_6;

    fn from_inner(inner: Self::Inner) -> Self {
        Self(inner)
    }
}

impl Neg for HumanAmount {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0) // Panics when the inner value is `i128::MIN`.
    }
}

impl Sub for HumanAmount {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

// --------------------------------- USD value ---------------------------------

/// A value in USD, in human unit. E.g. a value of 1.234 means US$1.234.
///
/// Technically, this is a special case of `HumanAmount`, as it's the human amount
/// of the asset USD. However, to differentiate with the amount of crypto assets,
/// we create the type specifically for USD.
#[grug::derive(Serde, Borsh)]
#[derive(Copy, PartialOrd, Ord)]
pub struct UsdValue(Dec128_6);

impl FromInner for UsdValue {
    type Inner = Dec128_6;

    fn from_inner(inner: Self::Inner) -> Self {
        Self(inner)
    }
}

// ----------------------------------- Ratio -----------------------------------

/// A ratio between two values.
#[grug::derive(Borsh)]
#[derive(Copy, PartialOrd, Ord)]
pub struct Ratio<N, D = N> {
    inner: Dec128_6,
    _numerator: PhantomData<N>,
    _denominator: PhantomData<D>,
}

impl<N, D> Ratio<N, D> {
    pub const HALF: Self = Self::new(Dec128_6::raw(Int128::new(500_000)));
    pub const ONE: Self = Self::new(Dec128_6::ONE);

    pub const fn new(inner: Dec128_6) -> Self {
        Self {
            inner,
            _numerator: PhantomData,
            _denominator: PhantomData,
        }
    }

    pub fn checked_add(self, rhs: Self) -> MathResult<Self> {
        self.inner.checked_add(rhs.inner).map(Self::new)
    }

    pub fn checked_sub(self, rhs: Self) -> MathResult<Self> {
        self.inner.checked_sub(rhs.inner).map(Self::new)
    }

    pub fn checked_mul<T>(self, rhs: Ratio<T, Self>) -> MathResult<T>
    where
        T: FromInner<Inner = Dec128_6>,
    {
        self.inner.checked_mul(rhs.inner).map(T::from_inner)
    }
}

impl<N, D> Ratio<N, D>
where
    N: Ord,
    D: Ord,
{
    /// Bound the value between `[min, max]` (both inclusive).
    pub fn clamp(self, min: Self, max: Self) -> Self {
        self.max(min).min(max)
    }
}

impl<N, D> Neg for Ratio<N, D> {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.inner) // Panics when the inner value is `i128::MIN`.
    }
}

impl<N, D> FromInner for Ratio<N, D> {
    type Inner = Dec128_6;

    fn from_inner(inner: Self::Inner) -> Self {
        Self::new(inner)
    }
}

impl<N, D> serde::ser::Serialize for Ratio<N, D> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.inner.serialize(serializer)
    }
}

impl<'de, N, D> serde::de::Deserialize<'de> for Ratio<N, D> {
    fn deserialize<DS>(deserializer: DS) -> Result<Self, DS::Error>
    where
        DS: serde::Deserializer<'de>,
    {
        serde::de::Deserialize::deserialize(deserializer).map(Self::new)
    }
}

/// The price of an asset, in human unit. E.g. a value of 1.234 means US$1.234
/// per 1 human unit of the asset.
pub type UsdPrice = Ratio<UsdValue, HumanAmount>;
