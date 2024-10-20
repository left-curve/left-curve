use grug::{Addr, Denom, Map};

pub const DENOM_ADMINS: Map<&Denom, Addr> = Map::new("denom");
