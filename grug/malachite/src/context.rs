use {
    crate::types,
    grug_types::Hash256,
    malachitebft_core_types::{Height, NilOrVal, Round, ValidatorSet, VoteType},
};

#[derive(Clone)]
pub struct Context;

impl malachitebft_core_types::Context for Context {
    type Address = types::Address;
    type Extension = ();
    type Height = types::Height;
    type Proposal = types::Proposal;
    type ProposalPart = types::ProposalPart;
    type SigningScheme = types::SigningScheme;
    type Validator = types::Validator;
    type ValidatorSet = types::ValidatorSet;
    type Value = types::Value;
    type Vote = types::Vote;

    fn select_proposer<'a>(
        &self,
        validator_set: &'a Self::ValidatorSet,
        height: Self::Height,
        round: Round,
    ) -> &'a Self::Validator {
        assert!(validator_set.count() > 0);
        assert!(round != Round::Nil && round.as_i64() >= 0);

        let proposer_index = {
            let height = height.as_u64() as usize;
            let round = round.as_i64() as usize;

            (height - 1 + round) % validator_set.count()
        };

        validator_set
            .get_by_index(proposer_index)
            .expect("proposer_index is valid")
    }

    fn new_proposal(
        &self,
        height: Self::Height,
        round: Round,
        value: Self::Value,
        pol_round: Round,
        address: Self::Address,
    ) -> Self::Proposal {
        types::Proposal {
            height,
            round,
            value,
            pol_round,
            validator_address: address,
        }
    }

    fn new_prevote(
        &self,
        height: Self::Height,
        round: Round,
        value_id: NilOrVal<Hash256>,
        address: Self::Address,
    ) -> Self::Vote {
        types::Vote::new(height, round, value_id, VoteType::Prevote, address)
    }

    fn new_precommit(
        &self,
        height: Self::Height,
        round: Round,
        value_id: NilOrVal<Hash256>,
        address: Self::Address,
    ) -> Self::Vote {
        types::Vote::new(height, round, value_id, VoteType::Precommit, address)
    }
}
