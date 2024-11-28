use {
    dango_types::vesting::{Config, Position, PositionIndex},
    grug::{Addr, Counter, IndexedMap, Item, MultiIndex},
};

pub const CONFIG: Item<Config> = Item::new("config");

pub const NEXT_POSITION_INDEX: Counter<PositionIndex> = Counter::new("index", 0, 1);

pub const POSITIONS: IndexedMap<PositionIndex, Position, PositionIndexes> =
    IndexedMap::new("position", PositionIndexes {
        user: MultiIndex::new(|_, position| position.user, "position", "position__user"),
    });

#[grug::index_list(PositionIndex, Position)]
pub struct PositionIndexes<'a> {
    pub user: MultiIndex<'a, PositionIndex, Addr, Position>,
}
