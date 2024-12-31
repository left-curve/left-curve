use {
    dango_testing::{setup_test_naive, Safe},
    dango_types::{
        account::{
            multi::{self, ParamUpdates, QueryProposalRequest, Status, Vote},
            single,
        },
        account_factory::{
            self, Account, AccountParams, QueryAccountRequest, QueryAccountsByUserRequest, Salt,
        },
    },
    grug::{
        btree_map, btree_set, Addr, Addressable, ChangeSet, Coins, HashExt, Inner, JsonSerExt,
        Message, NonEmpty, NonZero, ResultExt, Signer, Timestamp, Uint128,
    },
};

#[test]
fn safe() {
    let (mut suite, mut accounts, codes, contracts) = setup_test_naive();

    // ----------------------------- Safe creation -----------------------------

    // Create a Safe with users 1-3 as members and the following parameters.
    let mut params = multi::Params {
        members: btree_map! {
            accounts.user1.username.clone() => NonZero::new(1).unwrap(),
            accounts.user2.username.clone() => NonZero::new(1).unwrap(),
            accounts.user3.username.clone() => NonZero::new(1).unwrap(),
        },
        voting_period: NonZero::new(Timestamp::from_seconds(30)).unwrap(),
        threshold: NonZero::new(2).unwrap(),
        // For the purpose of this test, the Safe doesn't have a timelock.
        timelock: None,
    };

    // Member 1 sends a transaction to create the Safe.
    suite
        .execute(
            &mut accounts.user1,
            contracts.account_factory,
            &account_factory::ExecuteMsg::RegisterAccount {
                params: AccountParams::Safe(params.clone()),
            },
            // Fund the Safe with some tokens.
            // The Safe will pay for gas fees, so it must have sufficient tokens.
            Coins::one("uusdc", 5_000_000).unwrap(),
        )
        .should_succeed();

    // Derive the Safe's address.
    // We have 10 genesis users, indexed from 0, so the Safe's index should be 10.
    let safe_address = Addr::derive(
        contracts.account_factory,
        codes.account_safe.to_bytes().hash256(),
        Salt { index: 10 }.into_bytes().as_slice(),
    );
    let mut safe = Safe::new(safe_address);

    // Query the account params.
    suite
        .query_wasm_smart(contracts.account_factory, QueryAccountRequest {
            address: safe.address(),
        })
        .should_succeed_and_equal(Account {
            index: 10,
            params: AccountParams::Safe(params.clone()),
        });

    // The account should be been registered under each member's username.
    for (member, index) in [
        (&accounts.user1, 1),
        (&accounts.user2, 2),
        (&accounts.user3, 3),
    ] {
        suite
            .query_wasm_smart(contracts.account_factory, QueryAccountsByUserRequest {
                username: member.username.clone(),
            })
            .should_succeed_and_equal(btree_map! {
                // Query response should include the user's own spot account as
                // well as the Safe.
                member.address() => Account {
                    index,
                    params: AccountParams::Spot(single::Params::new(
                        member.username.clone()
                    )),
                },
                safe.address() => Account {
                    index: 10,
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
            safe.with_signer(&accounts.user1),
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
        .should_succeed();

    // Member 2 votes YES.
    suite
        .execute(
            safe.with_signer(&accounts.user2),
            safe_address,
            &multi::ExecuteMsg::Vote {
                proposal_id: 1,
                voter: accounts.user2.username.clone(),
                vote: Vote::Yes,
                execute: false,
            },
            Coins::new(),
        )
        .should_succeed();

    // User 3 votes YES with auto-execute.
    // The proposal should pass and execute.
    suite
        .execute(
            safe.with_signer(&accounts.user3),
            safe_address,
            &multi::ExecuteMsg::Vote {
                proposal_id: 1,
                voter: accounts.user3.username.clone(),
                vote: Vote::Yes,
                execute: true,
            },
            Coins::new(),
        )
        .should_succeed();

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
    // adding a new member (using `owner`) and removing one (using `user3`).
    let updates = ParamUpdates {
        members: ChangeSet::new(
            btree_map! {
                accounts.owner.username.clone() => NonZero::new(1).unwrap(),
            },
            btree_set! {
                accounts.user3.username.clone(),
            },
        )
        .unwrap(),
        voting_period: None,
        threshold: None,
    };

    suite
        .execute(
            safe.with_signer(&accounts.user1),
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
        .should_succeed();

    // Members 2 and 3 votes on the proposal (without auto-execute).
    for member in [&accounts.user2, &accounts.user3] {
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
            .should_succeed();
    }

    // Proposal should be in the "passed" state.
    suite
        .query_wasm_smart(safe.address(), QueryProposalRequest { proposal_id: 2 })
        .should_succeed_and(|prop| matches!(prop.status, Status::Passed { .. }));

    // Member 1 executes the proposal.
    suite
        .execute(
            safe.with_signer(&accounts.user1),
            safe_address,
            &multi::ExecuteMsg::Execute { proposal_id: 2 },
            Coins::new(),
        )
        .should_succeed();

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
            index: 10,
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
            username: accounts.user3.username.clone(),
        })
        .should_succeed_and(|accounts| !accounts.contains_key(&safe.address()));

    // ---------------------------- Failed proposal ----------------------------

    // Member 1 makes a proposal.
    suite
        .execute(
            safe.with_signer(&accounts.user1),
            safe_address,
            &multi::ExecuteMsg::Propose {
                title: "nothing".to_string(),
                description: None,
                messages: vec![],
            },
            Coins::new(),
        )
        .should_succeed();

    // Member 2 and owner vote against it.
    for member in [&accounts.user2, &accounts.owner] {
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
            .should_succeed();
    }

    // The proposal should be in the "failed" state.
    suite
        .query_wasm_smart(safe.address(), QueryProposalRequest { proposal_id: 3 })
        .should_succeed_and(|prop| prop.status == Status::Failed);

    // Attempting to execute the proposal should fail.
    suite
        .send_message(
            safe.with_signer(&accounts.user1),
            Message::execute(
                safe_address,
                &multi::ExecuteMsg::Execute { proposal_id: 3 },
                Coins::new(),
            )
            .unwrap(),
        )
        .should_fail_with_error("proposal isn't passed or timelock hasn't elapsed");

    // -------------------------- Unauthorized voting --------------------------

    // Member 1 makes a proposal. This should be proposal 4.
    suite
        .execute(
            safe.with_signer(&accounts.user1),
            safe_address,
            &multi::ExecuteMsg::Propose {
                title: "nothing".to_string(),
                description: None,
                messages: vec![],
            },
            Coins::new(),
        )
        .should_succeed();

    // accounts.user4 attempts to vote, impersonating member 2.
    // Since accounts.user4 doesn't actual know member 2's private key, the tx will be
    // signed by accounts.user4's private key.
    // There are a few variables to consider:
    // - the `voter` field in `ExecuteMsg::Vote`
    // - the `username` field in the metadata
    // - the `key_hash` field in the metadata
    // We test all 2**3 = 8 combinations.
    for (voter, username, key_hash, error) in [
        // First, in `dango_account_safe::authenticate`, the contract checks the
        // voter in the execute message matches the username in the metadata.
        // If not the same, the tx already fails here.
        (
            accounts.user4.username.clone(),
            accounts.user2.username.clone(),
            accounts.user4.first_key_hash(),
            "can't vote with a different username".to_string(),
        ),
        (
            accounts.user4.username.clone(),
            accounts.user2.username.clone(),
            accounts.user2.first_key_hash(),
            "can't vote with a different username".to_string(),
        ),
        (
            accounts.user2.username.clone(),
            accounts.user4.username.clone(),
            accounts.user4.first_key_hash(),
            "can't vote with a different username".to_string(),
        ),
        (
            accounts.user2.username.clone(),
            accounts.user4.username.clone(),
            accounts.user2.first_key_hash(),
            "can't vote with a different username".to_string(),
        ),
        // Then, the contract calls `dango_auth::authenticate`. The method first
        // checks the Safe is associated with the voter's username. That is, the
        // voter is a member of the Safe.
        (
            accounts.user4.username.clone(),
            accounts.user4.username.clone(),
            accounts.user4.first_key_hash(),
            format!(
                "account {} isn't associated with user `{}`",
                safe.address(),
                accounts.user4.username
            ),
        ),
        (
            accounts.user4.username.clone(),
            accounts.user4.username.clone(),
            accounts.user2.first_key_hash(),
            format!(
                "account {} isn't associated with user `{}`",
                safe.address(),
                accounts.user4.username
            ),
        ),
        // Now we know the voter and username must both be that of a member.
        (
            accounts.user2.username.clone(),
            accounts.user2.username.clone(),
            accounts.user4.first_key_hash(),
            format!("key hash {} not found", accounts.user4.first_key_hash()),
        ),
        (
            accounts.user2.username.clone(),
            accounts.user2.username.clone(),
            accounts.user2.first_key_hash(),
            "signature is unauthentic".to_string(),
        ),
    ] {
        // Sign the tx with accounts.user4s's private key.
        let mut tx = safe
            .with_signer(&accounts.user4)
            .with_nonce(12) // TODO: nonce isn't incremented if auth fails... should we make sure it increments?
            .sign_transaction(
                NonEmpty::new_unchecked(vec![Message::execute(
                    safe_address,
                    &multi::ExecuteMsg::Vote {
                        proposal_id: 4,
                        voter,
                        vote: Vote::Yes,
                        execute: false,
                    },
                    Coins::new(),
                )
                .unwrap()]),
                &suite.chain_id,
                suite.default_gas_limit,
            )
            .unwrap();

        tx.data.as_object_mut().unwrap().insert(
            "username".to_string(),
            username.to_json_value().unwrap().into_inner(),
        );

        tx.credential
            .as_object_mut()
            .and_then(|obj| obj.get_mut("standard"))
            .and_then(|val| val.as_object_mut())
            .map(|standard| {
                standard.insert(
                    "key_hash".to_string(),
                    key_hash.to_json_value().unwrap().into_inner(),
                )
            });

        suite.send_transaction(tx).should_fail_with_error(error);
    }
}
