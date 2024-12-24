use {
    grug::{Addr, Denom, Item, Map},
    hyperlane_types::{mailbox::Domain, Addr32},
};

pub const MAILBOX: Item<Addr> = Item::new("mailbox");

// (denom, destination_domain) => recipient
pub const ROUTES: Map<(&Denom, Domain), Addr32> = Map::new("route");

// (destination_domain, sender) => denom
pub const REVERSE_ROUTES: Map<(Domain, Addr32), Denom> = Map::new("collateral");
