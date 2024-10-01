use {
    crate::amm::{Config, Pool, PoolId, PoolParams},
    grug::{Coin, Coins, Uint256, UniqueVec},
    std::collections::BTreeMap,
};

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub config: Config,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Create a new trading pool with the given parameters.
    CreatePool(PoolParams),
    /// Perform a swap.
    Swap {
        // Note: the route must not contain any loop. We make sure of this by
        // using a `UniqueVec`.
        route: UniqueVec<PoolId>,
        minimum_output: Option<Uint256>,
    },
    /// Provide liquidity to a trading pool.
    ProvideLiquidity {
        pool_id: PoolId,
        minimum_output: Option<Uint256>,
    },
    /// Withdraw liquidity from a trading pool.
    WithdrawLiquidity { pool_id: PoolId },
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the AMM's global configuration.
    #[returns(Config)]
    Config {},
    /// Query the state of a single pool by ID.
    #[returns(Pool)]
    Pool { pool_id: PoolId },
    /// Enumerate the states of all pools.
    #[returns(BTreeMap<PoolId, Pool>)]
    Pools {
        start_after: Option<PoolId>,
        limit: Option<u32>,
    },
    /// Simulate the output of a swap.
    #[returns(SwapOutcome)]
    Simulate {
        input: Coin,
        route: UniqueVec<PoolId>,
    },
}

/// The outcome of performing a swap.
#[grug::derive(Serde)]
pub struct SwapOutcome {
    /// The amount of coin to be returned to the trader.
    pub output: Coin,
    /// The amount of fee paid to the protocol's token stakers.
    pub protocol_fee: Coin,
    /// The amount of fee paid to liquidity providers.
    pub liquidity_fees: Coins,
}
