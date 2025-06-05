use {
    crate::{
        context::Context,
        ctx,
        types::wrapper::{BNilOrVal, BRound, BSignedExtension, BVoteType},
    },
    grug::Hash256,
    malachitebft_core_types::{NilOrVal, Round, SignedExtension, VoteType},
};

#[grug::derive(Borsh)]
#[derive(PartialOrd, Ord)]
pub struct Vote {
    height: ctx!(Height),
    round: BRound,
    value: BNilOrVal<Hash256>,
    vote_type: BVoteType,
    validator_address: ctx!(Address),
    extension: Option<BSignedExtension>,
}

impl Vote {
    pub fn new(
        height: ctx!(Height),
        round: Round,
        value: NilOrVal<ctx!(Value::Id)>,
        vote_type: VoteType,
        validator_address: ctx!(Address),
    ) -> Self {
        Self {
            height,
            round: BRound(round),
            value: BNilOrVal(value),
            vote_type: BVoteType(vote_type),
            validator_address,
            extension: None,
        }
    }
}

impl malachitebft_core_types::Vote<Context> for Vote {
    fn height(&self) -> ctx!(Height) {
        self.height
    }

    fn round(&self) -> malachitebft_core_types::Round {
        self.round.0
    }

    fn take_value(self) -> NilOrVal<ctx!(Value::Id)> {
        self.value.0
    }

    fn vote_type(&self) -> VoteType {
        self.vote_type.0
    }

    fn validator_address(&self) -> &ctx!(Address) {
        &self.validator_address
    }

    fn extension(&self) -> Option<&SignedExtension<Context>> {
        self.extension.as_ref().map(|extension| &extension.0)
    }

    fn take_extension(&mut self) -> Option<SignedExtension<Context>> {
        self.extension.take().map(|extension| extension.0)
    }

    fn extend(mut self, extension: SignedExtension<Context>) -> Self {
        self.extension = Some(BSignedExtension(extension));
        self
    }

    fn value(&self) -> &NilOrVal<ctx!(Value::Id)> {
        &self.value.0
    }
}
