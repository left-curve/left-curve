use {
    dango_types::warp::{RateLimit, Route},
    grug::{Addr, Denom, Item, Map, Uint128},
    hyperlane_types::{Addr32, mailbox::Domain},
    std::collections::BTreeMap,
};

pub const MAILBOX: Item<Addr> = Item::new("mailbox");

pub const RATE_LIMITS: Item<BTreeMap<Denom, RateLimit>> = Item::new("rate_limits");

// (denom, destination_domain) => (recipient, withdrawal_fee)
//
// Used for outbound.
pub const ROUTES: Map<(&Denom, Domain), Route> = Map::new("route");

// (destination_domain, sender) => denom
//
// Used for inbound.
//
// `sender` means the Warp contract address on the destination domain.
pub const REVERSE_ROUTES: Map<(Domain, Addr32), Denom> = Map::new("reverse_route");

pub const OUTBOUND_QUOTAS: Map<&Denom, Uint128> = Map::new("outbound_quota");
