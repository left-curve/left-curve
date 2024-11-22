use {
    dango_types::vesting::Position,
    grug::{Addr, Counter, IndexedMap, MultiIndex},
};

pub const NEXT_POSITION_INDEX: Counter<u64> = Counter::new("index", 0, 1);

pub const POSITIONS: IndexedMap<u64, Position, PositionIndexes> =
    IndexedMap::new("position", PositionIndexes {
        user: MultiIndex::new(|_, position| position.user, "position", "position__user"),
    });

#[grug::index_list(u64, Position)]
pub struct PositionIndexes<'a> {
    pub user: MultiIndex<'a, u64, Addr, Position>,
}
