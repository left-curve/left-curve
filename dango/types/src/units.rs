//! In smart contracts, when we come across a number, let's say 123.456, it's
//! sometimes confusing what it means -- does it mean the USD value ("notional value")
//! of an asset, or the quantity of an asset? If it's a quantity, is it in the
//! human unit (e.g. BTC), or the base unit (satoshi or 1e-8 BTC)?
//! To avoid the confusion, we define a Rust type for each of these types of number.

use {
    grug::{Dec128_6, Uint128},
    std::marker::PhantomData,
};

/// A quantity of asset in _base unit_. E.g. a value of 1234, in the context of
/// USDC, which has 6 decimal places, means 1234 uusdc or 0.001234 USDC.
///
/// In Dango, this is used to denote the quantity of the settlement currency for
/// perpetual futures contracts, and tokenized shares in the counterparty vault.
#[grug::derive(Serde, Borsh)]
pub struct BaseAmount(Uint128);

/// A quantity of asset in _human unit_. E.g. a value of 1.234, in the context of
/// BTC, means 1.234 BTC (not BTC's base unit, which is satoshi or 1e-8 BTC).
///
/// In Dango, this is used to denote the quantity of perpetual futures contracts.
/// E.g. +1.234 units of the BTCUSD-PERP contract indicates a long exposure to
/// 1.234 BTC.
///
/// This value can be negative, in which case it represents a short position.
#[grug::derive(Serde, Borsh)]
pub struct HumanAmount(Dec128_6);

/// A value in USD, in human unit. E.g. a value of 1.234 means US$1.234.
#[grug::derive(Serde, Borsh)]
pub struct UsdValue(Dec128_6);

/// A ratio between two values.
#[grug::derive(Borsh)]
pub struct Ratio<N, D = N> {
    fraction: Dec128_6,
    _numerator: PhantomData<N>,
    _denominator: PhantomData<D>,
}

impl<N, D> serde::ser::Serialize for Ratio<N, D> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.fraction.serialize(serializer)
    }
}

impl<'de, N, D> serde::de::Deserialize<'de> for Ratio<N, D> {
    fn deserialize<DS>(deserializer: DS) -> Result<Self, DS::Error>
    where
        DS: serde::Deserializer<'de>,
    {
        Ok(Ratio {
            fraction: serde::de::Deserialize::deserialize(deserializer)?,
            _numerator: PhantomData,
            _denominator: PhantomData,
        })
    }
}

/// The price of an asset, in human unit. E.g. a value of 1.234 means US$1.234
/// per 1 human unit of the asset.
pub type UsdPrice = Ratio<UsdValue, HumanAmount>;
