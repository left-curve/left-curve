use {
    dango_types::vesting::{Position, Schedule},
    grug::{Addr, Item, Map},
};

pub const UNLOCKING_SCHEDULE: Item<Schedule> = Item::new("unlocking_schedule");

pub const POSITIONS: Map<Addr, Position> = Map::new("position");
