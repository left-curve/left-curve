//! This is a quick and dirty implementation. In the long term, a better idea
//! may be to introduce a generic into `Coins`: `Coins<T>` where `T` is either
//! `Uint128` or `Udec128`.

use {
    crate::{Coins, Denom, StdError, StdResult},
    borsh::{BorshDeserialize, BorshSerialize},
    grug_math::{Dec, FixedPoint, IsZero, Number, NumberConst},
    serde::{Deserialize, Serialize},
    std::{
        collections::{BTreeMap, btree_map},
        fmt::{self, Display},
    },
};

/// Like `Coin` but the amount is a decimal.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct DecCoin<const S: u32> {
    pub denom: Denom,
    pub amount: Dec<u128, S>,
}

impl<const S: u32> From<(Denom, Dec<u128, S>)> for DecCoin<S> {
    fn from((denom, amount): (Denom, Dec<u128, S>)) -> Self {
        Self { denom, amount }
    }
}

#[derive(
    Serialize, Deserialize, BorshSerialize, BorshDeserialize, Default, Debug, Clone, PartialEq, Eq,
)]
pub struct DecCoins<const S: u32>(BTreeMap<Denom, Dec<u128, S>>);

impl<const S: u32> DecCoins<S> {
    pub const EMPTY_DEC_COINS_STR: &'static str = "[]";

    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn is_non_empty(&self) -> bool {
        !self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Insert a new coin to the `Coins`.
    pub fn insert<T>(&mut self, dec_coin: T) -> StdResult<&mut Self>
    where
        T: TryInto<DecCoin<S>>,
        StdError: From<T::Error>,
        Dec<u128, S>: FixedPoint<u128> + NumberConst,
    {
        let dec_coin = dec_coin.try_into()?;

        let Some(amount) = self.0.get_mut(&dec_coin.denom) else {
            // If the denom doesn't exist, and we are increasing by a non-zero
            // amount: just create a new record, and we are done.
            if dec_coin.amount.is_non_zero() {
                self.0.insert(dec_coin.denom, dec_coin.amount);
            }

            return Ok(self);
        };

        amount.checked_add_assign(dec_coin.amount)?;

        Ok(self)
    }

    /// Insert all coins from another `Coins`.
    pub fn insert_many(&mut self, dec_coins: DecCoins<S>) -> StdResult<&mut Self>
    where
        Dec<u128, S>: FixedPoint<u128> + NumberConst,
    {
        for (denom, amount) in dec_coins {
            self.insert((denom, amount))?;
        }

        Ok(self)
    }

    pub fn into_coins_floor(self) -> Coins
    where
        Dec<u128, S>: FixedPoint<u128> + NumberConst,
    {
        let map = self
            .0
            .into_iter()
            .filter_map(|(denom, amount)| {
                let amount = amount.into_int_floor(); // Important: floor the amount.
                if amount.is_non_zero() {
                    Some((denom, amount))
                } else {
                    None
                }
            })
            .collect();
        Coins::new_unchecked(map)
    }

    pub fn into_coins_ceil(self) -> Coins
    where
        Dec<u128, S>: FixedPoint<u128> + NumberConst,
    {
        let map = self
            .0
            .into_iter()
            .filter_map(|(denom, amount)| {
                let amount = amount.into_int_ceil(); // Important: ceil the amount.
                if amount.is_non_zero() {
                    Some((denom, amount))
                } else {
                    None
                }
            })
            .collect();
        Coins::new_unchecked(map)
    }
}

impl<'a, const S: u32> IntoIterator for &'a DecCoins<S> {
    type IntoIter = btree_map::Iter<'a, Denom, Dec<u128, S>>;
    type Item = (&'a Denom, &'a Dec<u128, S>);

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<const S: u32> IntoIterator for DecCoins<S> {
    type IntoIter = btree_map::IntoIter<Denom, Dec<u128, S>>;
    type Item = (Denom, Dec<u128, S>);

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<const S: u32> fmt::Display for DecCoins<S>
where
    Dec<u128, S>: Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // special case: empty string
        if self.is_empty() {
            return f.write_str(Self::EMPTY_DEC_COINS_STR);
        }

        let s = self
            .into_iter()
            .map(|(denom, amount)| format!("{denom}:{amount}"))
            .collect::<Vec<_>>()
            .join(",");

        f.write_str(&s)
    }
}
