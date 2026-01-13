use {dango_types::account_factory::UserIndex, grug::Map};

/// Maps a referee to their referrer.
pub const REFEREE: Map<UserIndex, UserIndex> = Map::new("referee");

/// Maps a referrer to the count of their referees.
pub const REFEREE_COUNT: Map<UserIndex, u32> = Map::new("referee_count");
