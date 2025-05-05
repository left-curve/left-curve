use {
    dango_types::perps::{PerpsMarketParams, PerpsMarketState, PerpsVaultState},
    grug::{Addr, Denom, Item, Map, Uint128},
};

/// The perps vault state.
pub const PERPS_VAULT: Item<PerpsVaultState> = Item::new("perps_vault");

/// Perps vault deposits. The key is the owner address and the value is their vault shares.
pub const PERPS_VAULT_DEPOSITS: Map<&Addr, Uint128> = Map::new("perps_vault_deposits");

/// The perps markets. The key is the denom of the market.
pub const PERPS_MARKETS: Map<&Denom, PerpsMarketState> = Map::new("perps_market");

/// The perps market params. The key is the denom of the market.
pub const PERPS_MARKET_PARAMS: Map<&Denom, PerpsMarketParams> = Map::new("perps_market_params");
