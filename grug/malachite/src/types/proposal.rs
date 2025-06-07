use {
    crate::{context::Context, ctx},
    grug::{BorshSerExt, SignData, StdError},
    k256::sha2::Sha256,
    malachitebft_core_types::Round,
};

#[grug::derive(Borsh)]
pub struct Proposal {
    pub height: ctx!(Height),
    pub round: Round,
    pub value: ctx!(Value),
    pub pol_round: Round,
    pub validator_address: ctx!(Address),
}

impl Proposal {
    pub fn new(
        height: ctx!(Height),
        round: Round,
        value: ctx!(Value),
        pol_round: Round,
        validator_address: ctx!(Address),
    ) -> Self {
        Self {
            height,
            round,
            value,
            pol_round,
            validator_address,
        }
    }
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

impl SignData for Proposal {
    type Error = StdError;
    type Hasher = Sha256;

    fn to_prehash_sign_data(&self) -> Result<Vec<u8>, Self::Error> {
        self.to_borsh_vec()
    }
}
