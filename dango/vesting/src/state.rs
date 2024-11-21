use {
    dango_types::vesting::Position,
    grug::{Addr, Counter, IndexedMap, MultiIndex},
};

pub const POSITION_INDEX: Counter<u64> = Counter::new("position_index", 0, 1);

pub const POSITIONS: IndexedMap<u64, Position, PositionIndexes> =
    IndexedMap::new("positions", PositionIndexes {
        user: MultiIndex::new(|_, position| position.user, "positions", "positions_user"),
    });

#[grug::index_list(u64, Position)]
pub struct PositionIndexes<'a> {
    pub user: MultiIndex<'a, u64, Addr, Position>,
}
