use {
    grug::{Addr, Denom, Item, Map},
    hyperlane_types::{Addr32, mailbox::Domain},
};

pub const MAILBOX: Item<Addr> = Item::new("mailbox");

// (denom, destination_domain) => (recipient, withdrawal_fee)
//
// Used for outbound.
pub const ROUTES: Map<(&Denom, Domain), Addr32> = Map::new("route");

// (destination_domain, sender) => denom
//
// Used for inbound.
//
// `sender` means the Warp contract address on the destination domain.
pub const REVERSE_ROUTES: Map<(Domain, Addr32), Denom> = Map::new("reverse_route");
