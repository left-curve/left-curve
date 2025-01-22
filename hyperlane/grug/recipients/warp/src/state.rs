use {
    grug::{Addr, Denom, Item, Map},
    hyperlane_types::{
        mailbox::Domain,
        recipients::warp::{RateLimit, Route},
        Addr32,
    },
};

pub const MAILBOX: Item<Addr> = Item::new("mailbox");

// (denom, destination_domain) => (recipient, withdrawal_fee)
pub const ROUTES: Map<(&Denom, Domain), Route> = Map::new("route");

// (destination_domain, sender) => denom
pub const REVERSE_ROUTES: Map<(Domain, Addr32), Denom> = Map::new("collateral");

pub const RATE_LIMIT: Map<&Denom, RateLimit> = Map::new("rate_limit");
