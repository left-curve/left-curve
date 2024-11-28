use {
    dango_types::vesting::{Position, PositionIndex, Schedule},
    grug::{Addr, Counter, IndexedMap, Item, MultiIndex},
};

pub const UNLOCKING_SCHEDULE: Item<Schedule> = Item::new("unlocking_schedule");

pub const NEXT_POSITION_INDEX: Counter<PositionIndex> = Counter::new("index", 0, 1);

pub const POSITIONS: IndexedMap<PositionIndex, Position, PositionIndexes> =
    IndexedMap::new("position", PositionIndexes {
        user: MultiIndex::new(|_, position| position.user, "position", "position__user"),
    });

#[grug::index_list(PositionIndex, Position)]
pub struct PositionIndexes<'a> {
    pub user: MultiIndex<'a, PositionIndex, Addr, Position>,
}
