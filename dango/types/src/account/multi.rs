use {
    crate::{account_factory::UserIndex, auth::Nonce},
    anyhow::anyhow,
    grug::{ChangeSet, Duration, Inner, Message, NonZero, Timestamp},
    std::collections::{BTreeMap, BTreeSet},
};

/// Identifier of a proposal.
pub type ProposalId = u32;

/// The number of votes a member has.
///
/// E.g. if a user has a power of 2, then each vote this member casts is counted
/// as two votes.
pub type Power = u32;

/// Parameters of a multi-signature account.
#[grug::derive(Serde, Borsh)]
pub struct Params {
    /// Users who can votes in this multisig, and their respective voting power.
    pub members: BTreeMap<UserIndex, NonZero<Power>>,
    /// The period of time since a proposal's creation when votes can be casted.
    pub voting_period: NonZero<Duration>,
    /// The minimum number of YES votes a proposal must receive in order to pass.
    /// Must be between 1 and the total power across all members (inclusive).
    pub threshold: NonZero<Power>,
    /// The minimum delay after a proposal is passed before it can be executed.
    pub timelock: Option<NonZero<Duration>>,
}

impl Params {
    /// Find the voting power of a user. Error if the user doesn't have power.
    pub fn power_of(&self, user_index: UserIndex) -> anyhow::Result<Power> {
        self.members
            .get(&user_index)
            .map(|power| power.into_inner())
            .ok_or_else(|| {
                anyhow!("user `{user_index}` is not authorized to create or vote in this proposal")
            })
    }

    /// Sum up the total voting power across all members.
    pub fn total_power(&self) -> Power {
        self.members.values().map(|power| power.into_inner()).sum()
    }

    /// Apply a set of updates to self.
    pub fn apply_updates(&mut self, updates: ParamUpdates) {
        for member in updates.members.remove() {
            self.members.remove(member);
        }

        for (member, power) in updates.members.into_add() {
            self.members.insert(member, power);
        }

        if let Some(new) = updates.voting_period {
            self.voting_period = new;
        }

        if let Some(new) = updates.threshold {
            self.threshold = new;
        }
    }
}

/// A set of updates to be applied to a multi-signature account.
#[grug::derive(Serde)]
pub struct ParamUpdates {
    pub members: ChangeSet<UserIndex, NonZero<Power>>,
    pub voting_period: Option<NonZero<Duration>>,
    pub threshold: Option<NonZero<Power>>,
    // Note that we don't allow changing the timelock, which is an important
    // parameter in limiting admin power and minimizing trust in DeFi protocols.
}

// Note: we can't derive the Borsh traits on this because `Message`,
// which includes `serde_json::Value`, doesn't implement those traits.
#[grug::derive(Serde)]
pub struct Proposal {
    pub title: String,
    pub description: Option<String>,
    pub messages: Vec<Message>,
    pub status: Status,
}

/// Possible Statuses a proposal can be in.
#[grug::derive(Serde, Borsh)]
pub enum Status {
    /// The proposal is being voted on by members.
    Voting {
        /// Parameters for tallying the vote.
        ///
        /// These parameters can change at any time, so we save the parameters
        /// _at the time the proposal was created_ inside the proposal.
        params: Params,
        /// The time when voting period ends.
        until: Timestamp,
        /// Number of YES votes collected so far.
        yes: Power,
        /// Number of NO votes collected so far.
        no: Power,
    },
    /// The proposal has received equal or more YES votes than the multisig's
    /// threshold, and can be executed once the timelock (if any) is passed.
    Passed {
        /// The earliest time this proposal can be executed.
        execute_after: Timestamp,
    },
    /// The proposal has failed to receive a sufficient number of YES votes
    /// during its voting period.
    Failed,
    /// The proposal has passed and been executed.
    Executed,
}

/// A vote to a proposal.
///
/// We currently don't support "abstain" or "no with veto" votes. If you need
/// them, please let us know.
#[grug::derive(Serde, Borsh)]
#[derive(Copy)]
pub enum Vote {
    /// The member voices support for this proposal.
    Yes,
    /// The member voices opposition against this proposal.
    No,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Create a new proposal with the given title, descriptions, and messages.
    Propose {
        title: String,
        description: Option<String>,
        messages: Vec<Message>,
    },
    /// Vote on a proposal during its voting period.
    Vote {
        proposal_id: ProposalId,
        voter: UserIndex,
        vote: Vote,
        /// Immediately execute the proposal, if:
        /// - the vote is a YES vote, and
        /// - the vote causes the proposal to pass, and
        /// - there is no timelock.
        execute: bool,
    },
    /// Execute a proposal once it's passed and the timelock (if there is one)
    /// has elapsed.
    Execute { proposal_id: ProposalId },
}

// Note: we don't provide a method for querying the Safe's config. Query the
// account factory for this instead.
#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the most recent transaction nonces that have been recorded.
    #[returns(BTreeSet<Nonce>)]
    SeenNonces {},
    /// Query a proposal by ID.
    #[returns(Proposal)]
    Proposal { proposal_id: ProposalId },
    /// Enumerate all proposals.
    #[returns(BTreeMap<ProposalId, Proposal>)]
    Proposals {
        start_after: Option<ProposalId>,
        limit: Option<u32>,
    },
    /// Query a member's vote in a proposal.
    #[returns(Option<Vote>)]
    Vote {
        proposal_id: ProposalId,
        member: UserIndex,
    },
    /// Enumerate all votes in a proposal.
    #[returns(BTreeMap<UserIndex, Vote>)]
    Votes { proposal_id: ProposalId },
}
