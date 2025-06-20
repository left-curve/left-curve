use crate::{context::Context, ctx};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Validator {
    pub address: ctx!(Address),
    pub public_key: ctx!(SigningScheme::PublicKey),
    pub voting_power: u64,
}

impl malachitebft_core_types::Validator<Context> for Validator {
    fn address(&self) -> &ctx!(Address) {
        &self.address
    }

    fn public_key(&self) -> &ctx!(SigningScheme::PublicKey) {
        &self.public_key
    }

    fn voting_power(&self) -> malachitebft_core_types::VotingPower {
        self.voting_power
    }
}
