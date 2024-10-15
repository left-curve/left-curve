use {
    crate::{NEXT_PROPOSAL_ID, PROPOSALS, VOTES},
    anyhow::{bail, ensure},
    dango_auth::authenticate_tx,
    dango_types::{
        account::{
            multi::{ExecuteMsg, Proposal, ProposalId, Status, Vote},
            InstantiateMsg,
        },
        account_factory::{QueryAccountRequest, Username},
        auth::Metadata,
        config::ACCOUNT_FACTORY_KEY,
    },
    grug::{
        Addr, AuthCtx, AuthResponse, Inner, JsonDeExt, Message, MutableCtx, Response, StdResult, Tx,
    },
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, _msg: InstantiateMsg) -> anyhow::Result<Response> {
    let account_factory: Addr = ctx.querier.query_app_config(ACCOUNT_FACTORY_KEY)?;

    // Only the account factory can create new accounts.
    ensure!(
        ctx.sender == account_factory,
        "you don't have the right, O you don't have the right"
    );

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn authenticate(ctx: AuthCtx, tx: Tx) -> anyhow::Result<AuthResponse> {
    let metadata: Metadata = tx.data.clone().deserialize_json()?;

    // The only type of transaction a Safe account is allowed to emit is to
    // execute itself. Everything else needs to be done through proposals.
    // Additionally, if the action is proposing or voting, the proposer/voter's
    // username must match the transaction signer's username.
    for msg in &tx.msgs {
        match msg {
            Message::Execute { contract, msg, .. } if contract == ctx.contract => {
                if let ExecuteMsg::Vote { voter, .. } = msg.clone().deserialize_json()? {
                    ensure!(
                        voter == metadata.username,
                        "can't vote with a different username"
                    );
                }
            },
            _ => bail!("a Safe account can only execute itself"),
        }
    }

    authenticate_tx(ctx, tx, None, Some(metadata))?;

    Ok(AuthResponse::new().request_backrun(false))
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn receive(_ctx: MutableCtx) -> StdResult<Response> {
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::Propose {
            title,
            description,
            messages,
        } => propose(ctx, title, description, messages),
        ExecuteMsg::Vote {
            proposal_id,
            voter,
            vote,
            execute,
        } => do_vote(ctx, proposal_id, voter, vote, execute),
        ExecuteMsg::Execute { proposal_id } => execute_proposal(ctx, proposal_id),
    }
}

fn propose(
    ctx: MutableCtx,
    title: String,
    description: Option<String>,
    messages: Vec<Message>,
) -> anyhow::Result<Response> {
    let factory = ctx.querier.query_app_config(ACCOUNT_FACTORY_KEY)?;

    // Query the Safe's parameters from the account factory.
    //
    // These params can change at any time (via the Safe executing the factory's
    // `configure_safe` method).
    //
    // As such, we always use the params _at the time of the proposal's creation_
    // for tallying the proposal. Changes made to the Safe's params _after_ the
    // proposal's creation has no effect on it.
    let params = ctx
        .querier
        .query_wasm_smart(factory, QueryAccountRequest {
            address: ctx.contract,
        })?
        .params
        .as_safe();

    let proposal = Proposal {
        title,
        description,
        messages,
        status: Status::Voting {
            // Compute the voting period ending time.
            until: ctx.block.timestamp + params.voting_period.into_inner(),
            yes: 0,
            no: 0,
            params,
        },
    };

    // Increment proposal ID.
    let (proposal_id, _) = NEXT_PROPOSAL_ID.increment(ctx.storage)?;

    // Save the proposal.
    PROPOSALS.save(ctx.storage, proposal_id, &proposal)?;

    Ok(Response::new())
}

fn do_vote(
    ctx: MutableCtx,
    proposal_id: ProposalId,
    voter: Username,
    vote: Vote,
    execute: bool,
) -> anyhow::Result<Response> {
    let mut proposal = PROPOSALS.load(ctx.storage, proposal_id)?;

    // Ensure the voter hasn't already voted.
    // Unlike Cosmos SDK's x/gov module, we don't allow changing votes.
    // Whereas DAO voters sometimes change votes, this rarely happens for multisigs.
    ensure!(
        !VOTES.has(ctx.storage, (proposal_id, &voter)),
        "user `{voter}` has already voted in this proposal"
    );

    // Update vote count in the proposal status.
    let (params, yes, no) = match &mut proposal.status {
        Status::Voting {
            params,
            until,
            yes,
            no,
        } => {
            // Ensure voting period hasn't ended yet.
            ensure!(ctx.block.timestamp < *until, "voting period already ended");

            // Update the vote count.
            match vote {
                Vote::Yes => {
                    *yes += params.power_of(&voter)?;
                },
                Vote::No => {
                    *no += params.power_of(&voter)?;
                },
            }

            (&*params, *yes, *no)
        },
        _ => bail!("proposal is not in voting period"),
    };

    // Update the proposal's status, if possible.
    let msgs = if yes >= params.threshold.into_inner() {
        // The proposal has received sufficient number of YES votes. It passed.
        // If there's no timelock, and the voter requests to execute the proposal,
        // then execute it.
        match (params.timelock, execute) {
            // No timelock, execution required.
            // Execute the proposal.
            (None, true) => {
                proposal.status = Status::Executed;
                proposal.messages.clone()
            },
            // No timelock, execution not required.
            // Do not execute the proposal.
            (None, false) => {
                proposal.status = Status::Passed {
                    execute_after: ctx.block.timestamp,
                };

                vec![]
            },
            // Timelocked, execution not require.
            // Do not execute the proposal.
            (Some(timelock), false) => {
                proposal.status = Status::Passed {
                    execute_after: ctx.block.timestamp + timelock.into_inner(),
                };

                vec![]
            },
            // Timelocked, but execution required. Error.
            (Some(_), true) => {
                bail!("proposal passes but can't be executed due to timelock");
            },
        }
    } else if no + params.threshold.into_inner() > params.total_power() {
        // The proposal has received enough NO vote that it can't pass.
        proposal.status = Status::Failed;

        vec![]
    } else {
        // The proposal hasn't received enough vote to either pass or fail.
        vec![]
    };

    // Save the vote.
    VOTES.save(ctx.storage, (proposal_id, &voter), &vote)?;

    // Save the updated proposal.
    PROPOSALS.save(ctx.storage, proposal_id, &proposal)?;

    Ok(Response::new().add_messages(msgs))
}

fn execute_proposal(ctx: MutableCtx, proposal_id: ProposalId) -> anyhow::Result<Response> {
    let mut proposal = PROPOSALS.load(ctx.storage, proposal_id)?;

    // The propsal can only be executed if passed, and timelock has elapsed.
    let msgs = match proposal.status {
        Status::Passed { execute_after } if ctx.block.timestamp > execute_after => {
            proposal.status = Status::Executed;
            proposal.messages.clone()
        },
        _ => bail!("proposal isn't passed or timelock hasn't elapsed"),
    };

    // Save the updated proposal.
    PROPOSALS.save(ctx.storage, proposal_id, &proposal)?;

    Ok(Response::new().add_messages(msgs))
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_account_factory::ACCOUNTS_BY_USER,
        dango_types::{
            account::multi::{self, Params},
            account_factory::{self, Account, AccountParams},
            config::ACCOUNT_FACTORY_KEY,
        },
        grug::{
            btree_map, Addr, AuthMode, Coins, Duration, GenericResult, GenericResultExt, Hash,
            Json, JsonSerExt, MockContext, MockQuerier, NonZero, ResultExt, Timestamp, MOCK_BLOCK,
        },
        std::{collections::BTreeMap, str::FromStr},
        test_case::test_case,
    };

    /// Address of the account factory for use in the following tests.
    const ACCOUNT_FACTORY: Addr = Addr::mock(254);

    /// Address of the Safe for use in the following tests.
    const SAFE: Addr = Addr::mock(255);

    #[test]
    fn only_factory_can_instantiate() {
        let querier = MockQuerier::new()
            .with_app_config(ACCOUNT_FACTORY_KEY, ACCOUNT_FACTORY)
            .unwrap();

        let mut ctx = MockContext::new()
            .with_querier(querier)
            .with_contract(SAFE)
            .with_sender(Addr::mock(123))
            .with_funds(Coins::new());

        // Attempt to instantiate with a random address as sender. Should fail.
        {
            let res = instantiate(ctx.as_mutable(), InstantiateMsg {});
            assert!(res.is_err_and(|err| err.to_string().contains("you don't have the right")));
        }

        // Attempt to instantiate with the factory as sender. Should work.
        {
            ctx = ctx.with_sender(ACCOUNT_FACTORY);

            let res = instantiate(ctx.as_mutable(), InstantiateMsg {});
            assert!(res.is_ok());
        }
    }

    #[test]
    fn authenticating() {
        let member1 = Username::from_str("member1").unwrap();
        let member2 = Username::from_str("member2").unwrap();
        let member3 = Username::from_str("member3").unwrap();
        let non_member = Username::from_str("jake").unwrap();

        // Create a Safe with 3 signers.
        let querier = MockQuerier::new()
            .with_app_config(ACCOUNT_FACTORY_KEY, ACCOUNT_FACTORY)
            .unwrap()
            .with_raw_contract_storage(ACCOUNT_FACTORY, |storage| {
                for member in [&member1, &member2, &member3] {
                    ACCOUNTS_BY_USER.insert(storage, (member, SAFE)).unwrap();
                }
            });

        let mut ctx = MockContext::new()
            .with_querier(querier)
            .with_contract(SAFE)
            .with_mode(AuthMode::Finalize);

        // Someone who is not a member attempts to sender a tx with the Safe.
        // Should fail.
        {
            let res = authenticate(ctx.as_auth(), Tx {
                sender: SAFE,
                gas_limit: 1_000_000,
                msgs: vec![],
                data: Metadata {
                    username: non_member.clone(),
                    // The things below (key hash, sequence, credential) don't
                    // matter, because authentication should fail before we even
                    // reach the signature verification step.
                    key_hash: Hash::ZERO,
                    sequence: 0,
                }
                .to_json_value()
                .unwrap(),
                credential: Json::Null,
            });

            assert!(res.is_err_and(|err| err.to_string().contains(&format!(
                "account {SAFE} isn't associated with user `{non_member}`"
            ))));
        }

        // A member sends a tx, but it's doing something other than executing
        // the Safe itself. Should fail.
        {
            let res = authenticate(ctx.as_auth(), Tx {
                sender: SAFE,
                gas_limit: 1_000_000,
                msgs: vec![Message::transfer(Addr::mock(123), Coins::new()).unwrap()],
                data: Metadata {
                    username: member1,
                    key_hash: Hash::ZERO,
                    sequence: 0,
                }
                .to_json_value()
                .unwrap(),
                credential: Json::Null,
            });

            assert!(res.is_err_and(|err| err
                .to_string()
                .contains("a Safe account can only execute itself")));
        }

        // A member sends a tx, it's executing the Safe itself to vote in a
        // proposal, but voting with a different username. Should fail.
        {
            let res = authenticate(ctx.as_auth(), Tx {
                sender: SAFE,
                gas_limit: 1_000_000,
                msgs: vec![Message::execute(
                    SAFE,
                    &multi::ExecuteMsg::Vote {
                        proposal_id: 1,
                        voter: member2,
                        vote: Vote::Yes,
                        execute: false,
                    },
                    Coins::new(),
                )
                .unwrap()],
                data: Metadata {
                    username: member3,
                    key_hash: Hash::ZERO,
                    sequence: 0,
                }
                .to_json_value()
                .unwrap(),
                credential: Json::Null,
            });

            assert!(res.is_err_and(|err| err
                .to_string()
                .contains("can't vote with a different username")));
        }
    }

    #[test]
    fn creating_proposal() {
        let m1 = Username::from_str("member1").unwrap();
        let m2 = Username::from_str("member2").unwrap();
        let m3 = Username::from_str("member3").unwrap();
        let m4 = Username::from_str("member4").unwrap();

        let mut params = Params {
            members: btree_map! {
                m1.clone() => NonZero::new(1).unwrap(),
                m2.clone() => NonZero::new(1).unwrap(),
                m3.clone() => NonZero::new(1).unwrap(),
            },
            voting_period: NonZero::new(Duration::from_seconds(100)).unwrap(),
            threshold: NonZero::new(2).unwrap(),
            timelock: None,
        };

        // Need to make a clone of `params` so it can be moved into the closure.
        let params_clone = params.clone();
        let querier = MockQuerier::new()
            .with_app_config(ACCOUNT_FACTORY_KEY, ACCOUNT_FACTORY)
            .unwrap()
            .with_smart_query_handler(move |contract, data| {
                match (contract, data.deserialize_json().unwrap()) {
                    (ACCOUNT_FACTORY, account_factory::QueryMsg::Account { address: SAFE }) => {
                        Account {
                            index: 12345,
                            params: AccountParams::Safe(params_clone.clone()),
                        }
                        .to_json_value()
                        .into_generic_result()
                    },
                    _ => unreachable!(),
                }
            });

        let mut ctx = MockContext::new()
            .with_querier(querier)
            .with_contract(SAFE)
            .with_sender(SAFE)
            .with_funds(Coins::new());

        // Create the 1st proposal.
        {
            propose(ctx.as_mutable(), "first".to_string(), None, vec![]).unwrap();

            let proposal = PROPOSALS.load(&ctx.storage, 1).unwrap();

            assert_eq!(proposal.status, Status::Voting {
                params: params.clone(),
                until: ctx.block.timestamp + params.voting_period.into_inner(),
                yes: 0,
                no: 0
            });
        }

        // Change the params, then create the 2nd proposal.
        // The new proposal should use the updated params.
        params.members.insert(m4.clone(), NonZero::new(1).unwrap());

        ctx.update_querier(|querier| {
            querier.update_smart_query_handler(move |contract, data| {
                match (contract, data.deserialize_json().unwrap()) {
                    (ACCOUNT_FACTORY, account_factory::QueryMsg::Account { address: SAFE }) => {
                        Account {
                            index: 12345,
                            // Use the updated params here!
                            params: AccountParams::Safe(params.clone()),
                        }
                        .to_json_value()
                        .into_generic_result()
                    },
                    _ => unreachable!(),
                }
            })
        });

        // Create the 2nd proposal. It should use the updated params.
        {
            propose(ctx.as_mutable(), "second".to_string(), None, vec![]).unwrap();

            let proposal = PROPOSALS.load(&ctx.storage, 2).unwrap();

            // The newly added member `m4` should be included.
            assert!(matches!(
                proposal.status,
                Status::Voting { params, .. } if params.members.contains_key(&m4)
            ));
        }
    }

    #[test_case(
        Status::Voting {
            // Params doesn't matter for this test...
            params: Params {
                members: btree_map! {},
                voting_period: NonZero::new(Duration::from_seconds(100)).unwrap(),
                threshold: NonZero::new(2).unwrap(),
                timelock: None,
            },
            until: Timestamp::from_seconds(200),
            yes: 0,
            no: 0,
        },
        |result| result.is_err_and(|err| {
            err.to_string().contains("voting period already ended")
        });
        "voting period ended"
    )]
    #[test_case(
        Status::Passed { execute_after: Timestamp::from_seconds(100) },
        |result| result.is_err_and(|err| {
            err.to_string().contains("proposal is not in voting period")
        });
        "proposal already passed"
    )]
    #[test_case(
        Status::Executed,
        |result| result.is_err_and(|err| {
            err.to_string().contains("proposal is not in voting period")
        });
        "proposal already executed"
    )]
    #[test_case(
        Status::Failed,
        |result| result.is_err_and(|err| {
            err.to_string().contains("proposal is not in voting period")
        });
        "proposal already failed"
    )]
    fn voting_out_of_voting_period(
        status: Status,
        predicate: fn(anyhow::Result<Response>) -> bool,
    ) {
        let mut ctx = MockContext::new()
            .with_block_timestamp(Timestamp::from_seconds(300))
            .with_sender(SAFE)
            .with_funds(Coins::new());

        let voter = Username::from_str("member").unwrap();
        let vote = Vote::Yes;
        let proposal_id = 123;

        // Save the proposal.
        PROPOSALS
            .save(&mut ctx.storage, proposal_id, &Proposal {
                title: "title".to_string(),
                description: None,
                messages: vec![],
                status,
            })
            .unwrap();

        assert!(predicate(do_vote(
            ctx.as_mutable(),
            proposal_id,
            voter,
            vote,
            false,
        )))
    }

    #[test_case(
        btree_map! {
            Username::from_str("member1").unwrap() => Vote::Yes,
        },
        Username::from_str("member1").unwrap(),
        Vote::No,
        false,
        None,
        |result| result.is_err_and(|err| {
            err.to_string().contains("user `member1` has already voted in this proposal")
        }),
        // Transaction is reverted, no need to check proposal status.
        |_| true;
        "voting twice"
    )]
    #[test_case(
        btree_map! {},
        Username::from_str("jake").unwrap(),
        Vote::Yes,
        false,
        None,
        |result| result.is_err_and(|err| {
            err.to_string().contains("user `jake` is not authorized to create or vote in this proposal")
        }),
        |_| true;
        "non-member voting"
    )]
    #[test_case(
        btree_map! {
            Username::from_str("member1").unwrap() => Vote::Yes,
        },
        Username::from_str("member2").unwrap(),
        Vote::Yes,
        true,
        None,
        |response| response.is_ok_and(|res| res.submsgs.len() == 1),
        |proposal| proposal.status == Status::Executed;
        "proposal passes, no timelock, auth execute"
    )]
    #[test_case(
        btree_map! {
            Username::from_str("member1").unwrap() => Vote::Yes,
        },
        Username::from_str("member2").unwrap(),
        Vote::Yes,
        false,
        None,
        |result| result.is_ok_and(|res| res.submsgs.is_empty()),
        |proposal| proposal.status == Status::Passed { execute_after: MOCK_BLOCK.timestamp };
        "proposal passes, no timelock, manual execute"
    )]
    #[test_case(
        btree_map! {
            Username::from_str("member1").unwrap() => Vote::Yes,
        },
        Username::from_str("member2").unwrap(),
        Vote::Yes,
        true,
        Some(Duration::from_seconds(100)),
        |result| result.is_err_and(|err| {
            err.to_string().contains("proposal passes but can't be executed due to timelock")
        }),
        |_| true;
        "proposal passes, has timelock, auto execution rejected"
    )]
    #[test_case(
        btree_map! {
            Username::from_str("member1").unwrap() => Vote::Yes,
        },
        Username::from_str("member2").unwrap(),
        Vote::Yes,
        false,
        Some(Duration::from_seconds(100)),
        |result| result.is_ok_and(|res| res.submsgs.is_empty()),
        |proposal| {
            proposal.status == Status::Passed {
                // Current block time + timelock
                execute_after: MOCK_BLOCK.timestamp + Duration::from_seconds(100),
            }
        };
        "proposal passes, has timelock, manual execute"
    )]
    #[test_case(
        btree_map! {
            Username::from_str("member1").unwrap() => Vote::No,
        },
        Username::from_str("member2").unwrap(),
        Vote::No,
        false,
        None,
        |result| result.is_ok_and(|res| res.submsgs.is_empty()),
        |proposal| proposal.status == Status::Failed;
        "proposal fails prematurely"
    )]
    #[test_case(
        btree_map! {
            Username::from_str("member1").unwrap() => Vote::Yes,
        },
        Username::from_str("member2").unwrap(),
        Vote::No,
        false,
        None,
        |result| result.is_ok_and(|res| res.submsgs.is_empty()),
        |proposal| matches!(proposal.status, Status::Voting { yes: 1, no: 1, .. });
        "not enough vote to either pass or fail yet"
    )]
    fn voting(
        previous_votes: BTreeMap<Username, Vote>,
        voter: Username,
        vote: Vote,
        execute: bool,
        timelock: Option<Duration>,
        result_predicate: fn(anyhow::Result<Response>) -> bool,
        proposal_predicate: fn(Proposal) -> bool,
    ) {
        let member1 = Username::from_str("member1").unwrap();
        let member2 = Username::from_str("member2").unwrap();
        let member3 = Username::from_str("member3").unwrap();

        let members = btree_map! {
            member1.clone() => NonZero::new(1).unwrap(),
            member2.clone() => NonZero::new(1).unwrap(),
            member3.clone() => NonZero::new(1).unwrap(),
        };

        let previous_yes_votes = previous_votes
            .iter()
            .map(|(voter, vote)| {
                if *vote == Vote::Yes {
                    members.get(voter).unwrap().into_inner()
                } else {
                    0
                }
            })
            .sum();

        let previous_no_votes = previous_votes
            .iter()
            .map(|(voter, vote)| {
                if *vote == Vote::No {
                    members.get(voter).unwrap().into_inner()
                } else {
                    0
                }
            })
            .sum();

        let mut ctx = MockContext::new()
            .with_sender(SAFE)
            .with_funds(Coins::new());

        let proposal_id = 123;

        // Save the proposal.
        PROPOSALS
            .save(&mut ctx.storage, proposal_id, &Proposal {
                title: "title".to_string(),
                description: None,
                messages: vec![Message::transfer(
                    Addr::mock(123),
                    Coins::one("uusdc", 12_345).unwrap(),
                )
                .unwrap()],
                status: Status::Voting {
                    params: Params {
                        members,
                        voting_period: NonZero::new(Duration::from_seconds(100)).unwrap(),
                        threshold: NonZero::new(2).unwrap(),
                        timelock: timelock.map(|d| NonZero::new(d).unwrap()),
                    },
                    until: Timestamp::from_seconds(200),
                    yes: previous_yes_votes,
                    no: previous_no_votes,
                },
            })
            .unwrap();

        // Save previous votes.
        for (voter, vote) in previous_votes {
            VOTES
                .save(&mut ctx.storage, (proposal_id, &voter), &vote)
                .unwrap();
        }

        // Check the result matches expectation.
        assert!(result_predicate(do_vote(
            ctx.as_mutable(),
            proposal_id,
            voter.clone(),
            vote,
            execute,
        )));

        // Check the updated proposal status matches expectation.
        assert!(proposal_predicate(
            PROPOSALS.load(&ctx.storage, proposal_id).unwrap()
        ));
    }

    #[test_case(
        Status::Voting {
            params: Params {
                members: btree_map! {},
                voting_period: NonZero::new(Duration::from_seconds(100)).unwrap(),
                threshold: NonZero::new(1).unwrap(),
                timelock: None,
            },
            until: Timestamp::from_seconds(100),
            yes: 0,
            no: 0,
        },
        GenericResult::Err("proposal isn't passed or timelock hasn't elapsed".to_string());
        "proposal still voting"
    )]
    #[test_case(
        Status::Passed {
            execute_after: Timestamp::from_seconds(500),
        },
        GenericResult::Err("proposal isn't passed or timelock hasn't elapsed".to_string());
        "proposal passed but still in timelock"
    )]
    #[test_case(
        Status::Passed {
            execute_after: Timestamp::from_seconds(100),
        },
        GenericResult::Ok(Response::new());
        "proposal passed and timelock elapsed"
    )]
    #[test_case(
        Status::Failed,
        GenericResult::Err("proposal isn't passed or timelock hasn't elapsed".to_string());
        "proposal failed"
    )]
    #[test_case(
        Status::Executed,
        GenericResult::Err("proposal isn't passed or timelock hasn't elapsed".to_string());
        "proposal already executed"
    )]
    fn executing(status: Status, expect: GenericResult<Response>) {
        let mut ctx = MockContext::new()
            .with_block_timestamp(Timestamp::from_seconds(200))
            .with_sender(SAFE)
            .with_funds(Coins::new());

        let proposal_id = 123;

        PROPOSALS
            .save(&mut ctx.storage, proposal_id, &Proposal {
                title: "title".to_string(),
                description: None,
                messages: vec![],
                status,
            })
            .unwrap();

        execute_proposal(ctx.as_mutable(), proposal_id).should_match(expect);
    }
}
