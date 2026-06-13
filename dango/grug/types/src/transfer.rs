use {
    crate::{Addr, Coin, Coins, DecCoin, DecCoins, Denom, Message, StdResult},
    grug_math::{Dec, FixedPoint, NumberConst, Uint128},
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
        self.batch
            .entry(address)
            .or_default()
            .insert(Coin { denom, amount })
            .map(|_| ())
    }

    pub fn insert_many(&mut self, address: Addr, coins: Coins) -> StdResult<()> {
        self.batch
            .entry(address)
            .or_default()
            .insert_many(coins)
            .map(|_| ())
    }

    pub fn into_batch(self) -> BTreeMap<Addr, Coins> {
        self.batch
    }

    pub fn into_message(self) -> Message {
        Message::Transfer(self.batch)
    }
}

impl<const S: u32> TransferBuilder<DecCoins<S>>
where
    Dec<u128, S>: FixedPoint<u128> + NumberConst,
{
    pub fn insert(&mut self, address: Addr, denom: Denom, amount: Dec<u128, S>) -> StdResult<()> {
        self.batch
            .entry(address)
            .or_default()
            .insert(DecCoin { denom, amount })
            .map(|_| ())
    }

    pub fn insert_many(&mut self, address: Addr, dec_coins: DecCoins<S>) -> StdResult<()> {
        self.batch
            .entry(address)
            .or_default()
            .insert_many(dec_coins)
            .map(|_| ())
    }

    pub fn into_batch(self) -> BTreeMap<Addr, Coins> {
        self.batch
            .into_iter()
            .filter_map(|(addr, dec_coins)| {
                // Round _down_ decimal amount to integer.
                let coins = dec_coins.into_coins_floor();
                if coins.is_non_empty() {
                    Some((addr, coins))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn into_message(self) -> Message {
        Message::Transfer(self.into_batch())
    }
}
