use crate::{context::Context, ctx};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Validator {
    address: ctx!(Address),
    public_key: ctx!(SigningScheme::PublicKey),
    voting_power: u64,
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
