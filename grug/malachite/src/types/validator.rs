use crate::{
    context::Context,
    types::{Address, PublicKey},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Validator {
    address: Address,
    public_key: PublicKey,
    voting_power: u64,
}

impl malachitebft_core_types::Validator<Context> for Validator {
    fn address(&self) -> &<Context as malachitebft_core_types::Context>::Address {
        &self.address
    }

    fn public_key(&self) -> &malachitebft_core_types::PublicKey<Context> {
        &self.public_key
    }

    fn voting_power(&self) -> malachitebft_core_types::VotingPower {
        self.voting_power
    }
}
