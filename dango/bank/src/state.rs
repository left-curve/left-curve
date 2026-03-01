use {
    dango_types::bank::Metadata,
    grug::{Addr, Coins, Denom, IndexedMap, Item, Map, MultiIndex, Part, Uint128},
};

pub const NAMESPACE_OWNERS: Map<&Part, Addr> = Map::new("namespace_owner");

pub const METADATAS: Map<&Denom, Metadata> = Map::new("metadata");

pub const SUPPLIES: Map<&Denom, Uint128> = Map::new("supply");

pub const BALANCES: Map<(&Addr, &Denom), Uint128> = Map::new("balance");

/// The perps contract is allowed to overdraw its balance in the settlement
/// currency. See the comment in `bank_execute` for rationale.
pub const PERP_DEFICIT: Item<Uint128> = Item::new("perp_deficit");

// (sender, recipient) -> coins
pub const ORPHANED_TRANSFERS: IndexedMap<(Addr, Addr), Coins, OrphanedTransferIndexes> =
    IndexedMap::new("orphaned_transfer", OrphanedTransferIndexes {
        recipient: MultiIndex::new(
            |(_, recipient), _| *recipient,
            "orphaned_transfer",
            "orphaned_transfer__recipient",
        ),
    });

#[grug::index_list((Addr, Addr), Coins)]
pub struct OrphanedTransferIndexes<'a> {
    pub recipient: MultiIndex<'a, (Addr, Addr), Addr, Coins>,
}
