use {
    crate::{Addr, Coin, Coins, DecCoin, DecCoins, Denom, Message, NonEmpty, StdResult},
    grug_math::{Dec, FixedPoint, IsZero, NumberConst, Uint128},
    std::collections::BTreeMap,
};

#[derive(Default, Debug)]
pub struct TransferBuilder<T: Default = Coins> {
    batch: BTreeMap<Addr, T>,
}

impl<T> TransferBuilder<T>
where
    T: Default,
{
    pub fn new() -> Self {
        Self {
            batch: BTreeMap::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.batch.is_empty()
    }

    pub fn is_non_empty(&self) -> bool {
        !self.batch.is_empty()
    }

    pub fn get_mut(&mut self, user: Addr) -> &mut T {
        self.batch.entry(user).or_default()
    }
}

impl TransferBuilder<Coins> {
    pub fn insert(&mut self, address: Addr, denom: Denom, amount: Uint128) -> StdResult<()> {
        // No-op if the amount is zero.
        if amount.is_zero() {
            return Ok(());
        }

        self.batch
            .entry(address)
            .or_default()
            .insert(Coin { denom, amount })
            .map(|_| ())
    }

    pub fn insert_many(&mut self, address: Addr, coins: Coins) -> StdResult<()> {
        // No-op if the coins are empty.
        if coins.is_empty() {
            return Ok(());
        }

        self.batch
            .entry(address)
            .or_default()
            .insert_many(coins)
            .map(|_| ())
    }

    /// Returns `None` if the transfer is empty.
    pub fn into_batch(self) -> Option<NonEmpty<BTreeMap<Addr, NonEmpty<Coins>>>> {
        let batch = self
            .batch
            .into_iter()
            .map(|(to, coins)| {
                // Note: our insertion logic ensures that `coins` is non-empty.
                (to, NonEmpty::new_unchecked(coins))
            })
            .collect();

        NonEmpty::new(batch).ok()
    }

    /// Returns `None` if the transfer is empty.
    pub fn into_message(self) -> Option<Message> {
        self.into_batch().map(Message::Transfer)
    }
}

impl<const S: u32> TransferBuilder<DecCoins<S>>
where
    Dec<u128, S>: FixedPoint<u128> + NumberConst,
{
    pub fn insert(&mut self, address: Addr, denom: Denom, amount: Dec<u128, S>) -> StdResult<()> {
        // No-op if the amount is zero.
        if amount.is_zero() {
            return Ok(());
        }

        self.batch
            .entry(address)
            .or_default()
            .insert(DecCoin { denom, amount })
            .map(|_| ())
    }

    pub fn insert_many(&mut self, address: Addr, dec_coins: DecCoins<S>) -> StdResult<()> {
        // No-op if the coins are empty.
        if dec_coins.is_empty() {
            return Ok(());
        }

        self.batch
            .entry(address)
            .or_default()
            .insert_many(dec_coins)
            .map(|_| ())
    }

    /// Returns `None` if the transfer is empty.
    pub fn into_batch(self) -> Option<NonEmpty<BTreeMap<Addr, NonEmpty<Coins>>>> {
        let batch = self
            .batch
            .into_iter()
            .filter_map(|(addr, dec_coins)| {
                // Round _down_ decimal amount to integer.
                let coins = dec_coins.into_coins_floor();

                // Unlike with `Coins`, it's possible that a `DecCoins` is non-empty,
                // but after rounding down, it becomes empty. So we need to filter.
                NonEmpty::new(coins).ok().map(|coins| (addr, coins))
            })
            .collect();

        NonEmpty::new(batch).ok()
    }

    /// Returns `None` if the transfer is empty.
    pub fn into_message(self) -> Option<Message> {
        self.into_batch().map(Message::Transfer)
    }
}
