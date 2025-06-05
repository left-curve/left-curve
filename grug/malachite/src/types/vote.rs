use {
    crate::{context::Context, ctx},
    grug::Hash256,
    malachitebft_core_types::{NilOrVal, Round, SignedExtension, VoteType},
};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Vote {
    height: ctx!(Height),
    round: Round,
    value: NilOrVal<Hash256>,
    vote_type: VoteType,
    validator_address: ctx!(Address),
    extension: Option<SignedExtension<Context>>,
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
            round,
            value,
            vote_type,
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
        self.round
    }

    fn take_value(self) -> NilOrVal<ctx!(Value::Id)> {
        self.value
    }

    fn vote_type(&self) -> VoteType {
        self.vote_type
    }

    fn validator_address(&self) -> &ctx!(Address) {
        &self.validator_address
    }

    fn extension(&self) -> Option<&SignedExtension<Context>> {
        self.extension.as_ref()
    }

    fn take_extension(&mut self) -> Option<SignedExtension<Context>> {
        self.extension.take()
    }

    fn extend(mut self, extension: SignedExtension<Context>) -> Self {
        self.extension = Some(extension);
        self
    }

    fn value(&self) -> &NilOrVal<ctx!(Value::Id)> {
        &self.value
    }
}
