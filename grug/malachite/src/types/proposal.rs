use {
    crate::{context::Context, ctx},
    malachitebft_core_types::Round,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Proposal {
    pub height: ctx!(Height),
    pub round: Round,
    pub value: ctx!(Value),
    pub pol_round: Round,
    pub validator_address: ctx!(Address),
}

impl malachitebft_core_types::Proposal<Context> for Proposal {
    fn height(&self) -> ctx!(Height) {
        self.height
    }

    fn round(&self) -> Round {
        self.round
    }

    fn value(&self) -> &ctx!(Value) {
        &self.value
    }

    fn take_value(self) -> ctx!(Value) {
        self.value
    }

    fn pol_round(&self) -> Round {
        self.pol_round
    }

    fn validator_address(&self) -> &ctx!(Address) {
        &self.validator_address
    }
}
