//! This is a quick and dirty implementation. In the long term, a better idea
//! may be to introduce a generic into `Coins`: `Coins<T>` where `T` is either
//! `Uint128` or `Udec128`.

use {
    crate::{Coins, Denom, StdError, StdResult},
    grug_math::{IsZero, MathError, NextNumber, Number, PrevNumber, Udec128, Udec256, Uint128},
    std::{
        collections::{BTreeMap, btree_map},
        fmt,
    },
};

/// Like `Coin` but the amount is a decimal.
pub struct DecCoin {
    pub denom: Denom,
    pub amount: Udec256,
}

impl From<(Denom, Udec128)> for DecCoin {
    fn from((denom, amount): (Denom, Udec128)) -> Self {
        Self {
            denom,
            amount: amount.into_next(),
        }
    }
}

impl From<(Denom, Udec256)> for DecCoin {
    fn from((denom, amount): (Denom, Udec256)) -> Self {
        Self { denom, amount }
    }
}

impl TryFrom<(Denom, Uint128)> for DecCoin {
    type Error = StdError;

    fn try_from((denom, amount): (Denom, Uint128)) -> Result<Self, Self::Error> {
        let amount = amount.into_next().checked_into_dec()?;
        Ok(Self { denom, amount })
    }
}

#[derive(Default, Debug)]
pub struct DecCoins(BTreeMap<Denom, Udec256>);

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

    pub fn into_coins_floor(self) -> Result<Coins, MathError> {
        let map = self
            .0
            .into_iter()
            .filter_map(|(denom, amount)| {
                // Important: floor the amount.
                amount
                    .into_int_floor()
                    .checked_into_prev()
                    .map(|a| {
                        if a.is_non_zero() {
                            Some((denom, a))
                        } else {
                            None
                        }
                    })
                    .transpose()
            })
            .collect::<Result<_, _>>()?;
        Ok(Coins::new_unchecked(map))
    }

    pub fn into_coins_ceil(self) -> Result<Coins, MathError> {
        let map = self
            .0
            .into_iter()
            .filter_map(|(denom, amount)| {
                // Important: ceil the amount.
                amount
                    .into_int_ceil()
                    .checked_into_prev()
                    .map(|a| {
                        if amount.is_non_zero() {
                            Some((denom, a))
                        } else {
                            None
                        }
                    })
                    .transpose()
            })
            .collect::<Result<_, _>>()?;
        Ok(Coins::new_unchecked(map))
    }
}

impl<'a> IntoIterator for &'a DecCoins {
    type IntoIter = btree_map::Iter<'a, Denom, Udec256>;
    type Item = (&'a Denom, &'a Udec256);

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl IntoIterator for DecCoins {
    type IntoIter = btree_map::IntoIter<Denom, Udec256>;
    type Item = (Denom, Udec256);

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
