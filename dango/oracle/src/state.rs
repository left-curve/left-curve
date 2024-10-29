use {dango_types::oracle::GuardianSetInfo, grug::Map};

pub const GUARDIAN_SETS: Map<u8, GuardianSetInfo> = Map::new("guardian_sets");
