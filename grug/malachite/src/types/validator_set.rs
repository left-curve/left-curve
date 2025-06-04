use malachitebft_core_types::{Validator as MalachiteValidator, VotingPower};

use crate::{
    context::Context,
    types::{Address, Validator},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatorSet(Vec<Validator>);

impl malachitebft_core_types::ValidatorSet<Context> for ValidatorSet {
    fn count(&self) -> usize {
        self.0.len()
    }

    fn total_voting_power(&self) -> VotingPower {
        self.0.iter().map(|v| v.voting_power()).sum()
    }

    fn get_by_address(&self, address: &Address) -> Option<&Validator> {
        self.0.iter().find(|v| v.address() == address)
    }

    fn get_by_index(&self, index: usize) -> Option<&Validator> {
        self.0.get(index)
    }
}
