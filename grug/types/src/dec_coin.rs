use {
    crate::{Coins, Denom, StdError, StdResult},
    grug_math::{IsZero, Number, Udec128},
    std::collections::{BTreeMap, btree_map},
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

#[derive(Default)]
pub struct DecCoins(BTreeMap<Denom, Udec128>);

impl DecCoins {
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
}

impl IntoIterator for DecCoins {
    type IntoIter = btree_map::IntoIter<Denom, Udec128>;
    type Item = (Denom, Udec128);

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl From<DecCoins> for Coins {
    fn from(dec_coins: DecCoins) -> Self {
        let map = dec_coins
            .0
            .into_iter()
            .filter_map(|(denom, amount)| {
                let amount = amount.into_int();
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
