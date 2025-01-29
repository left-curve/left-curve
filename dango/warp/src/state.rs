use {
    dango_types::warp::{Alloyed, Route},
    grug::{Addr, Denom, IndexedMap, Item, Map, UniqueIndex},
    hyperlane_types::{mailbox::Domain, Addr32},
};

pub const MAILBOX: Item<Addr> = Item::new("mailbox");

// (denom, destination_domain) => (recipient, withdrawal_fee)
pub const ROUTES: Map<(&Denom, Domain), Route> = Map::new("route");

// (destination_domain, sender) => denom
pub const REVERSE_ROUTES: Map<(Domain, Addr32), Denom> = Map::new("collateral");

/// underlay_denom => alloyed
pub const ALLOYED: IndexedMap<Denom, Alloyed, AlloyedIndex> =
    IndexedMap::new("alloyed", AlloyedIndex {
        alloyed_domain: UniqueIndex::new(
            |_, alloyed| (alloyed.alloyed_denom.clone(), alloyed.destination_domain),
            "alloyed",
            "alloyed__domain",
        ),
    });

#[grug::index_list(Denom, Alloyed)]
pub struct AlloyedIndex<'a> {
    /// (alloyed_denom, destination_domain) => alloyed
    pub alloyed_domain: UniqueIndex<'a, Denom, (Denom, Domain), Alloyed>,
}
