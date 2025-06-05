use std::collections::BTreeMap;

use grug::{Addr, Denom, Int128, Uint128};

use super::{PerpsMarketParams, PerpsMarketState, PerpsPositionResponse, PerpsVaultState};

pub const INITIAL_SHARES_PER_TOKEN: Uint128 = Uint128::new(1_000_000);

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    /// The denom of the perps vault.
    pub perps_vault_denom: Denom,
    /// The parameters for the perps markets.
    pub perps_market_params: BTreeMap<Denom, PerpsMarketParams>,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Deposit tokens into the perps vault.
    Deposit {},
    /// Withdraw tokens from the perps vault.
    Withdraw {
        /// The amount of shares to withdraw.
        shares: Uint128,
    },
    /// Batch update orders.
    BatchUpdateOrders {
        /// The orders to update. A map from denom to position size change (positive for long, negative for short).
        orders: BTreeMap<Denom, Int128>,
    },
    /// Update the parameters for the perps markets.
    UpdatePerpsMarketParams {
        /// The new parameters for the perps markets.
        params: BTreeMap<Denom, PerpsMarketParams>,
    },
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Get the perps vault state.
    #[returns(PerpsVaultState)]
    PerpsVaultState {},

    /// Get the perps market params for a specific denom.
    #[returns(PerpsMarketParams)]
    PerpsMarketParamsForDenom {
        /// The denom of the perps market.
        denom: Denom,
    },

    /// Get the perps market params for all denoms.
    #[returns(BTreeMap<Denom, PerpsMarketParams>)]
    PerpsMarketParams {
        /// The maximum number of market params to return.
        limit: Option<u32>,
        /// The starting denom to return market params from.
        start_after: Option<Denom>,
    },

    /// Get the vault share of a user.
    #[returns(Uint128)]
    VaultSharesForUser {
        /// The address of the user.
        address: Addr,
    },

    /// Get the vault share for all users
    #[returns(BTreeMap<Addr, Uint128>)]
    VaultShares {
        /// The maximum number of results to return.
        limit: Option<u32>,
        /// The starting address to return shares from.
        start_after: Option<Addr>,
    },

    /// Get the perps market state for a specific denom.
    #[returns(PerpsMarketState)]
    PerpsMarketStateForDenom {
        /// The denom of the perps market.
        denom: Denom,
    },

    /// Get the perps market state for all denoms.
    #[returns(BTreeMap<Denom, PerpsMarketState>)]
    PerpsMarketStates {
        /// The maximum number of results to return.
        limit: Option<u32>,
        /// The starting denom to return market state from.
        start_after: Option<Denom>,
    },

    /// Get the perps positions for a user.
    #[returns(BTreeMap<Denom, PerpsPositionResponse>)]
    PerpsPositionsForUser {
        /// The address of the user.
        address: Addr,
    },

    /// Get the perps positions for all users.
    #[returns(BTreeMap<Addr, BTreeMap<Denom, PerpsPositionResponse>>)]
    PerpsPositions {
        /// The maximum number of results to return.
        limit: Option<u32>,
        /// The starting user address and denom to return positions from.
        start_after: Option<(Addr, Denom)>,
    },
}
