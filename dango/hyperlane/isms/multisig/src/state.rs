use {
    dango_hyperlane_types::{isms::multisig::ValidatorSet, mailbox::Domain},
    dango_storage::Map,
};

pub const VALIDATOR_SETS: Map<Domain, ValidatorSet> = Map::new("validator_set");
