use {
    grug::{Addr, Denom, Item, Map},
    hyperlane_types::Addr32,
};

pub const OWNER: Item<Addr> = Item::new("owner");

pub const MAILBOX: Item<Addr> = Item::new("mailbox");

// (denom, destination_domain) => recipient
pub const ROUTES: Map<(&Denom, u32), Addr32> = Map::new("route");

// (destination_domain, sender) => denom
pub const REVERSE_ROUTES: Map<(u32, Addr32), Denom> = Map::new("collateral");
