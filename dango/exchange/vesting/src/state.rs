use {
    dango_primitives::Addr,
    dango_storage::{Item, Map},
    dango_types::vesting::{Position, Schedule},
};

pub const UNLOCKING_SCHEDULE: Item<Schedule> = Item::new("unlocking_schedule");

pub const POSITIONS: Map<Addr, Position> = Map::new("position");
