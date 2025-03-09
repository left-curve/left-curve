use {
    crate::{PROPOSALS, VOTES},
    dango_auth::query_seen_nonces,
    dango_types::{
        account::multi::{Proposal, ProposalId, QueryMsg, Status, Vote},
        account_factory::Username,
    },
    grug::{Bound, DEFAULT_PAGE_LIMIT, ImmutableCtx, Json, JsonSerExt, Order, StdResult, Storage},
    std::collections::BTreeMap,
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::SeenNonces {} => {
            let res = query_seen_nonces(ctx.storage)?;
            res.to_json_value()
        },
        QueryMsg::Proposal { proposal_id } => {
            let res = query_proposal(ctx, proposal_id)?;
            res.to_json_value()
        },
        QueryMsg::Proposals { start_after, limit } => {
            let res = query_proposals(ctx, start_after, limit)?;
            res.to_json_value()
        },
        QueryMsg::Vote {
            proposal_id,
            member,
        } => {
            let res = query_vote(ctx.storage, proposal_id, member)?;
            res.to_json_value()
        },
        QueryMsg::Votes { proposal_id } => {
            let res = query_votes(ctx.storage, proposal_id)?;
            res.to_json_value()
        },
    }
}

fn query_proposal(ctx: ImmutableCtx, proposal_id: ProposalId) -> StdResult<Proposal> {
    let mut proposal = PROPOSALS.load(ctx.storage, proposal_id)?;

    // If the proposal is in "voting" state, but voting period has already
    // finished, it means not enough vote is received. The proposal fails.
    if let Status::Voting { until, .. } = &proposal.status {
        if ctx.block.timestamp > *until {
            proposal.status = Status::Failed;
        }
    }

    Ok(proposal)
}

fn query_proposals(
    ctx: ImmutableCtx,
    start_after: Option<ProposalId>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<ProposalId, Proposal>> {
    let start = start_after.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    PROPOSALS
        .range(ctx.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| {
            let (proposal_id, mut proposal) = res?;

            if let Status::Voting { until, .. } = &proposal.status {
                if ctx.block.timestamp > *until {
                    proposal.status = Status::Failed;
                }
            }

            Ok((proposal_id, proposal))
        })
        .collect()
}

fn query_vote(
    storage: &dyn Storage,
    proposal_id: ProposalId,
    member: Username,
) -> StdResult<Option<Vote>> {
    VOTES.may_load(storage, (proposal_id, &member))
}

fn query_votes(
    storage: &dyn Storage,
    proposal_id: ProposalId,
) -> StdResult<BTreeMap<Username, Vote>> {
    VOTES
        .prefix(proposal_id)
        .range(storage, None, None, Order::Ascending)
        .collect()
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_types::account::multi::Params,
        grug::{MockContext, NonZero, Timestamp, btree_map},
        std::str::FromStr,
    };

    #[test]
    fn querying_proposal() {
        let mut ctx = MockContext::new();

        let proposal_id = 123;

        let proposal = Proposal {
            title: "title".to_string(),
            description: None,
            messages: vec![],
            status: Status::Voting {
                params: Params {
                    members: btree_map! {
                        Username::from_str("a").unwrap() => NonZero::new(1).unwrap(),
                        Username::from_str("b").unwrap() => NonZero::new(1).unwrap(),
                        Username::from_str("c").unwrap() => NonZero::new(1).unwrap(),
                    },
                    voting_period: NonZero::new(Timestamp::from_seconds(100)).unwrap(),
                    threshold: NonZero::new(2).unwrap(),
                    timelock: None,
                },
                until: Timestamp::from_seconds(100),
                yes: 1,
                no: 1,
            },
        };

        PROPOSALS
            .save(&mut ctx.storage, proposal_id, &proposal)
            .unwrap();

        // Attempt to query the proposal at timestamp 50.
        // Voting period hasn't ended yet, so proposal should be in "voting" status.
        {
            ctx.set_block_timestamp(Timestamp::from_seconds(50));

            let proposal = query_proposal(ctx.as_immutable(), proposal_id).unwrap();
            assert!(matches!(proposal.status, Status::Voting { .. }));
        }

        // Attempt to query the proposal at timestamp 120.
        // Voting period has ended, but the proposal didn't receive enough votes
        // to either pass or straightout fail. In this case, it fails.
        {
            ctx.set_block_timestamp(Timestamp::from_seconds(120));

            let proposal = query_proposal(ctx.as_immutable(), proposal_id).unwrap();
            assert_eq!(proposal.status, Status::Failed);
        }
    }
}
