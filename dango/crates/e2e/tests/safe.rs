use {
    dango_e2e::{setup_test, Factory, Safe, TestAccount},
    dango_types::{
        account::{
            multi::{self, ParamUpdates, QueryProposalRequest, Status, Vote},
            single,
        },
        account_factory::{
            self, Account, AccountParams, QueryAccountRequest, QueryAccountsByUserRequest, Salt,
        },
        mock_ibc_transfer, ChangeSet,
    },
    grug::{
        btree_map, btree_set, Addr, Addressable, Coins, HashExt, Message, NonZero, ResultExt,
        Timestamp, Uint128,
    },
};

#[test]
fn safe() {
    let (mut suite, mut accounts, codes, contracts) = setup_test().unwrap();

    // --------------------------------- Setup ---------------------------------

    // Onboard 4 users. They will be the members of the Safe.
    let [mut member1, member2, member3]: [TestAccount; 3] = ["member1", "member2", "member3"]
        .into_iter()
        .map(|username| {
            let user = TestAccount::new_random(username)
                .unwrap()
                .predict_address(
                    contracts.account_factory,
                    codes.account_spot.to_bytes().hash256(),
                    true,
                )
                .unwrap();

            suite
                .execute(
                    &mut accounts.relayer,
                    contracts.ibc_transfer,
                    &mock_ibc_transfer::ExecuteMsg::ReceiveTransfer {
                        recipient: user.address(),
                    },
                    Coins::one("uusdc", 100_000_000).unwrap(),
                )
                .unwrap();

            suite
                .execute(
                    &mut Factory::new(contracts.account_factory),
                    contracts.account_factory,
                    &account_factory::ExecuteMsg::RegisterUser {
                        username: user.username.clone(),
                        key: user.key,
                        key_hash: user.key_hash,
                    },
                    Coins::new(),
                )
                .unwrap();

            user
        })
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();

    // ----------------------------- Safe creation -----------------------------

    // Create a Safe with members 1-3 and the following parameters.
    let mut params = multi::Params {
        members: btree_map! {
            member1.username.clone() => NonZero::new(1).unwrap(),
            member2.username.clone() => NonZero::new(1).unwrap(),
            member3.username.clone() => NonZero::new(1).unwrap(),
        },
        voting_period: NonZero::new(Timestamp::from_seconds(30)).unwrap(),
        threshold: NonZero::new(2).unwrap(),
        // For the purpose of this test, the Safe doesn't have a timelock.
        timelock: None,
    };

    // Member 1 sends a transaction to create the Safe.
    suite
        .execute(
            &mut member1,
            contracts.account_factory,
            &account_factory::ExecuteMsg::RegisterAccount {
                params: AccountParams::Safe(params.clone()),
            },
            // Fund the Safe with some tokens.
            // The Safe will pay for gas fees, so it must have sufficient tokens.
            Coins::one("uusdc", 5_000_000).unwrap(),
        )
        .unwrap();

    // Derive the Safe's address.
    // We have 3 genesis users + 3 members, so the Safe's index should be 6.
    let safe_address = Addr::compute(
        contracts.account_factory,
        codes.account_safe.to_bytes().hash256(),
        Salt { index: 6 }.into_bytes().as_slice(),
    );
    let mut safe = Safe::new(safe_address);

    // Query the account params.
    suite
        .query_wasm_smart(contracts.account_factory, QueryAccountRequest {
            address: safe.address(),
        })
        .should_succeed_and_equal(Account {
            index: 6,
            params: AccountParams::Safe(params.clone()),
        });

    // The account should be been registered under each member's username.
    for (member, index) in [(&member1, 3), (&member2, 4), (&member3, 5)] {
        suite
            .query_wasm_smart(contracts.account_factory, QueryAccountsByUserRequest {
                username: member.username.clone(),
            })
            .should_succeed_and_equal(btree_map! {
                // Query response should include the user's own spot account as
                // well as the Safe.
                member.address() => Account {
                    index,
                    params: AccountParams::Spot(single::Params {
                        owner: member.username.clone()
                    }),
                },
                safe.address() => Account {
                    index: 6,
                    params: AccountParams::Safe(params.clone()),
                },
            });
    }

    // The Safe should have received tokens.
    suite
        .query_balance(&safe, "uusdc")
        .should_succeed_and_equal(Uint128::new(5_000_000));

    // ---------------------- Proposal with auto-execute -----------------------

    // Member 1 makes a proposal to transfer some tokens.
    suite
        .execute(
            safe.with_signer(&member1),
            safe_address,
            &multi::ExecuteMsg::Propose {
                title: "send 123 uusdc to owner".to_string(),
                description: None,
                messages: vec![Message::transfer(
                    accounts.owner.address(),
                    Coins::one("uusdc", 888_888).unwrap(),
                )
                .unwrap()],
            },
            Coins::new(),
        )
        .unwrap();

    // Member 2 votes YES.
    suite
        .execute(
            safe.with_signer(&member2),
            safe_address,
            &multi::ExecuteMsg::Vote {
                proposal_id: 1,
                voter: member2.username.clone(),
                vote: Vote::Yes,
                execute: false,
            },
            Coins::new(),
        )
        .unwrap();

    // Member 3 votes YES with auto-execute.
    // The proposal should pass and execute.
    suite
        .execute(
            safe.with_signer(&member3),
            safe_address,
            &multi::ExecuteMsg::Vote {
                proposal_id: 1,
                voter: member3.username.clone(),
                vote: Vote::Yes,
                execute: true,
            },
            Coins::new(),
        )
        .unwrap();

    // Proposal should be in the "executed" state.
    suite
        .query_wasm_smart(safe.address(), QueryProposalRequest { proposal_id: 1 })
        .should_succeed_and(|prop| prop.status == Status::Executed);

    // Ensure the tokens have been delivered.
    // Owner has 100_000_000_000 uusd to start, and now has received 888_888.
    suite
        .query_balance(&accounts.owner, "uusdc")
        .should_succeed_and_equal(Uint128::new(100_000_888_888));

    // --------------------- Proposal with manual execute ----------------------

    // Member 1 makes another proposal. This time to amend the members set,
    // adding a new member and removing one.
    let updates = ParamUpdates {
        members: ChangeSet::new(
            btree_map! {
                accounts.owner.username.clone() => NonZero::new(1).unwrap(),
            },
            btree_set! {
                member3.username.clone(),
            },
        )
        .unwrap(),
        voting_period: None,
        threshold: None,
    };

    suite
        .execute(
            safe.with_signer(&member1),
            safe_address,
            &multi::ExecuteMsg::Propose {
                title: "add owner as member".to_string(),
                description: None,
                messages: vec![Message::execute(
                    contracts.account_factory,
                    &account_factory::ExecuteMsg::ConfigureSafe {
                        updates: updates.clone(),
                    },
                    Coins::new(),
                )
                .unwrap()],
            },
            Coins::new(),
        )
        .unwrap();

    // Member 2 and 3 votes on the proposal (without auto-execute).
    for member in [&member2, &member3] {
        suite
            .execute(
                safe.with_signer(member),
                safe_address,
                &multi::ExecuteMsg::Vote {
                    proposal_id: 2,
                    voter: member.username.clone(),
                    vote: Vote::Yes,
                    execute: false,
                },
                Coins::new(),
            )
            .unwrap();
    }

    // Proposal should be in the "passed" state.
    suite
        .query_wasm_smart(safe.address(), QueryProposalRequest { proposal_id: 2 })
        .should_succeed_and(|prop| matches!(prop.status, Status::Passed { .. }));

    // Member 1 executes the proposal.
    suite
        .execute(
            safe.with_signer(&member1),
            safe_address,
            &multi::ExecuteMsg::Execute { proposal_id: 2 },
            Coins::new(),
        )
        .unwrap();

    // Proposal should now be in the "executed" state.
    suite
        .query_wasm_smart(safe.address(), QueryProposalRequest { proposal_id: 2 })
        .should_succeed_and(|prop| prop.status == Status::Executed);

    // Ensure the params have been amended.
    params.apply_updates(updates);

    suite
        .query_wasm_smart(contracts.account_factory, QueryAccountRequest {
            address: safe.address(),
        })
        .should_succeed_and_equal(Account {
            index: 6,
            params: AccountParams::Safe(params),
        });

    // The new member
    suite
        .query_wasm_smart(contracts.account_factory, QueryAccountsByUserRequest {
            username: accounts.owner.username.clone(),
        })
        .should_succeed_and(|accounts| accounts.contains_key(&safe.address()));

    // The removed member
    suite
        .query_wasm_smart(contracts.account_factory, QueryAccountsByUserRequest {
            username: member3.username.clone(),
        })
        .should_succeed_and(|accounts| !accounts.contains_key(&safe.address()));

    // ---------------------------- Failed proposal ----------------------------

    // Member 1 makes a proposal.
    suite
        .execute(
            safe.with_signer(&member1),
            safe_address,
            &multi::ExecuteMsg::Propose {
                title: "nothing".to_string(),
                description: None,
                messages: vec![],
            },
            Coins::new(),
        )
        .unwrap();

    // Members 2 and owner vote against it.
    for member in [&member2, &accounts.owner] {
        suite
            .execute(
                safe.with_signer(member),
                safe_address,
                &multi::ExecuteMsg::Vote {
                    proposal_id: 3,
                    voter: member.username.clone(),
                    vote: Vote::No,
                    execute: false,
                },
                Coins::new(),
            )
            .unwrap();
    }

    // The proposal should be in the "failed" state.
    suite
        .query_wasm_smart(safe.address(), QueryProposalRequest { proposal_id: 3 })
        .should_succeed_and(|prop| prop.status == Status::Failed);

    // Attempting to execute the proposal should fail.
    suite
        .send_message(
            safe.with_signer(&member1),
            Message::execute(
                safe_address,
                &multi::ExecuteMsg::Execute { proposal_id: 3 },
                Coins::new(),
            )
            .unwrap(),
        )
        .unwrap()
        .result
        .should_fail_with_error("proposal isn't passed or timelock hasn't elapsed");
}
