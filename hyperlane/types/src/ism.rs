use {
    grug::{HexBinary, HexByteArray},
    std::collections::{BTreeMap, BTreeSet},
};

#[grug::derive(Serde, Borsh)]
pub struct ValidatorSet {
    pub threshold: u8,
    pub validators: BTreeSet<HexByteArray<20>>,
}

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub validator_sets: BTreeMap<u32, ValidatorSet>,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Set validators for a domain.
    SetValidators {
        domain: u32,
        threshold: u8,
        validators: BTreeSet<HexByteArray<20>>,
    },
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the validator set for a domain.
    #[returns(ValidatorSet)]
    ValidatorSet { domain: u32 },
    /// Enumerate validator sets of all domains.
    #[returns(BTreeMap<u32, ValidatorSet>)]
    ValidatorSets {
        start_after: Option<u32>,
        limit: Option<u32>,
    },
    /// Verify a message.
    /// Return nothing is succeeds; throw error if fails.
    #[returns(())]
    Verify {
        raw_message: HexBinary,
        metadata: HexBinary,
    },
}
