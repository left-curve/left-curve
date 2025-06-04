use {
    crate::{
        context::Context,
        types::{Address, Height, Value},
    },
    malachitebft_core_types::Round,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Proposal {
    pub height: Height,
    pub round: Round,
    pub value: Value,
    pub pol_round: Round,
    pub validator_address: Address,
}

impl malachitebft_core_types::Proposal<Context> for Proposal {
    fn height(&self) -> Height {
        self.height
    }

    fn round(&self) -> Round {
        self.round
    }

    fn value(&self) -> &Value {
        &self.value
    }

    fn take_value(self) -> Value {
        self.value
    }

    fn pol_round(&self) -> Round {
        self.pol_round
    }

    fn validator_address(&self) -> &Address {
        &self.validator_address
    }
}
