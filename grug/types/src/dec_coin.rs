//! This is a quick and dirty implementation. In the long term, a better idea
//! may be to introduce a generic into `Coins`: `Coins<T>` where `T` is either
//! `Uint128` or `Udec128`.

use {
    crate::{Coins, Denom, StdError, StdResult},
    grug_math::{IsZero, Number, Udec128},
    std::{
        collections::{BTreeMap, btree_map},
        fmt,
    },
};

/// Like `Coin` but the amount is a decimal.
pub struct DecCoin {
    pub denom: Denom,
    pub amount: Udec128,
}

impl From<(Denom, Udec128)> for DecCoin {
    fn from((denom, amount): (Denom, Udec128)) -> Self {
        Self { denom, amount }
    }
}

#[derive(Default, Debug)]
pub struct DecCoins(BTreeMap<Denom, Udec128>);

impl DecCoins {
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
        T: TryInto<DecCoin>,
        StdError: From<T::Error>,
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
    pub fn insert_many(&mut self, dec_coins: DecCoins) -> StdResult<&mut Self> {
        for (denom, amount) in dec_coins {
            self.insert(DecCoin { denom, amount })?;
        }

        Ok(self)
    }

    pub fn into_coins_floor(self) -> Coins {
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

    pub fn into_coins_ceil(self) -> Coins {
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

impl<'a> IntoIterator for &'a DecCoins {
    type IntoIter = btree_map::Iter<'a, Denom, Udec128>;
    type Item = (&'a Denom, &'a Udec128);

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl IntoIterator for DecCoins {
    type IntoIter = btree_map::IntoIter<Denom, Udec128>;
    type Item = (Denom, Udec128);

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl fmt::Display for DecCoins {
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
