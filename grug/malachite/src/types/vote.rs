use {
    crate::{
        context::Context,
        types::{Address, Height},
    },
    grug_types::Hash256,
    malachitebft_core_types::{NilOrVal, Round, SignedExtension, VoteType},
};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Vote {
    height: Height,
    round: Round,
    value: NilOrVal<Hash256>,
    vote_type: VoteType,
    validator_address: Address,
    extension: Option<SignedExtension<Context>>,
}

impl Vote {
    pub fn new(
        height: Height,
        round: Round,
        value: NilOrVal<Hash256>,
        vote_type: VoteType,
        validator_address: Address,
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
    fn height(&self) -> <Context as malachitebft_core_types::Context>::Height {
        self.height
    }

    fn round(&self) -> malachitebft_core_types::Round {
        self.round
    }

    fn take_value(self) -> NilOrVal<Hash256> {
        self.value
    }

    fn vote_type(&self) -> VoteType {
        self.vote_type
    }

    fn validator_address(&self) -> &Address {
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

    fn value(&self) -> &NilOrVal<Hash256> {
        &self.value
    }
}
