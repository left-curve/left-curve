use {
    dango_types::amm::{Config, Pool, PoolId},
    grug::{Counter, Item, Map},
};

pub const CONFIG: Item<Config> = Item::new("config");

pub const NEXT_POOL_ID: Counter<PoolId> = Counter::new("next_pool_id", 1, 1);

pub const POOLS: Map<PoolId, Pool> = Map::new("pool");
