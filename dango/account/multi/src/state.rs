use {
    dango_types::{
        account::multi::{Proposal, ProposalId, Vote},
        account_factory::UserIndex,
    },
    grug::{Counter, Map, Serde},
};

pub const NEXT_PROPOSAL_ID: Counter<ProposalId> = Counter::new("next_proposal_id", 1, 1);

// Note: Have to use serde codec for this, because `Proposal` contains `Message`
// which contains `serde_json::Value` which doesn't implement Borsh traits.
pub const PROPOSALS: Map<ProposalId, Proposal, Serde> = Map::new("proposal");

pub const VOTES: Map<(ProposalId, UserIndex), Vote> = Map::new("vote");
