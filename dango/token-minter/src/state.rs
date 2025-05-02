use {
    dango_types::token_minter::{DestinationChain, RateLimit},
    grug::{Addr, Denom, Item, Map, Uint128},
    std::collections::BTreeMap,
};

pub const DENOMS: Map<&Denom, Addr> = Map::new("denom");

pub const FEES: Map<&Denom, Uint128> = Map::new("fee");

// underlying_denom => alloyed_denom
//
// E.g.
// - hyp/eth/usdc => hyp/all/eth
// - hyp/sol/usdc => hyp/all/eth
//
// Used for inbound.
pub const ALLOYS: Map<&Denom, Denom> = Map::new("alloy");

// (alloyed_denom, destination_domain) => underlying_denom
//
// E.g.
// - (hyp/all/eth, eth) => hyp/eth/usdc
// - (hyp/all/sol, sol) => hyp/sol/usdc
//
// Used for outbound.
pub const REVERSE_ALLOYS: Map<(&Denom, &DestinationChain), Denom> = Map::new("reverse_alloy");

pub const OUTBOUND_QUOTAS: Map<&Denom, Uint128> = Map::new("outbound_quota");

pub const RATE_LIMITS: Item<BTreeMap<Denom, RateLimit>> = Item::new("rate_limits");
