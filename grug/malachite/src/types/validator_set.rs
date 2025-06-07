use {
    crate::{context::Context, ctx, types::Address},
    malachitebft_core_types::{Validator as MalachiteValidator, VotingPower},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatorSet(Vec<ctx!(Validator)>);

impl malachitebft_core_types::ValidatorSet<Context> for ValidatorSet {
    fn count(&self) -> usize {
        self.0.len()
    }

    fn total_voting_power(&self) -> VotingPower {
        self.0.iter().map(|v| v.voting_power()).sum()
    }

    fn get_by_address(&self, address: &Address) -> Option<&ctx!(Validator)> {
        self.0.iter().find(|v| v.address() == address)
    }

    fn get_by_index(&self, index: usize) -> Option<&ctx!(Validator)> {
        self.0.get(index)
    }
}
