use {
    grug::Map,
    hyperlane_types::{ism::ValidatorSet, mailbox::Domain},
};

pub const VALIDATOR_SETS: Map<Domain, ValidatorSet> = Map::new("validator_set");
