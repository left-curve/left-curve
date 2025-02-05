use {
    dango_types::warp::Route,
    grug::{Addr, Denom, Item, Map},
    hyperlane_types::{mailbox::Domain, Addr32},
};

pub const MAILBOX: Item<Addr> = Item::new("mailbox");

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

// (underlying_denom, destination_domain) => alloyed_denom
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
pub const REVERSE_ALLOYS: Map<(&Denom, Domain), Denom> = Map::new("reverse_alloy");
