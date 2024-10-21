use {grug::Denom, grug_storage::Set};

pub const WHITELISTED_DENOMS: Set<Denom> = Set::new("whitelisted_denoms");
