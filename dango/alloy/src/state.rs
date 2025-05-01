use grug::{Denom, Map};

// underlying denom => alloyed denom
pub const UNDERLYING_TO_ALLOYED: Map<&Denom, Denom> = Map::new("a");

// alloyed denom => underlying denom
pub const ALLOYED_TO_UNDERLYING: Map<&Denom, Denom> = Map::new("b");
