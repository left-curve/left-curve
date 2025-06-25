use {
    crate::{Addr, Coin, Coins, Denom, Message, StdResult},
    grug_math::Uint128,
    std::collections::BTreeMap,
};

#[derive(Default, Debug)]
pub struct TransferBuilder {
    batch: BTreeMap<Addr, Coins>,
}

impl TransferBuilder {
    pub fn new() -> Self {
        Self {
            batch: BTreeMap::new(),
        }
    }

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

    pub fn is_empty(&self) -> bool {
        self.batch.is_empty()
    }

    pub fn is_non_empty(&self) -> bool {
        !self.batch.is_empty()
    }

    pub fn into_batch(self) -> BTreeMap<Addr, Coins> {
        self.batch
    }

    pub fn into_message(self) -> Message {
        Message::Transfer(self.batch)
    }
}
