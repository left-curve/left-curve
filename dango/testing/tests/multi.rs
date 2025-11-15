use {
    dango_account_factory::KEYS,
    dango_auth::MAX_NONCE_INCREASE,
    dango_genesis::Contracts,
    dango_testing::{
        Multi, TestAccount, TestAccounts, TestSuite, constants::GENESIS_USER_COUNT,
        setup_test_naive,
    },
    dango_types::{
        account::{
            multi::{self, QueryProposalRequest, QueryVoteRequest, Status, Vote},
            single,
        },
        account_factory::{
            self, Account, AccountParamUpdates, AccountParams, QueryAccountRequest,
            QueryAccountsByUserRequest, Salt, Username,
        },
        auth::Key,
        constants::usdc,
    },
    grug::{
        Addr, Addressable, ChangeSet, Coins, Duration, Empty, Hash256, HashExt, Inner, JsonSerExt,
        Message, NonEmpty, NonZero, QuerierExt, ResultExt, Signer, StdError, Tx, Uint128,
        btree_map, btree_set,
    },
    grug_app::NaiveProposalPreparer,
};

// Create a multi-signature account with users 1-3 as members.
fn setup_multi_test<'a>() -> (
    TestSuite<NaiveProposalPreparer>,
    TestAccounts,
    Contracts,
    Multi<'a>,
    multi::Params,
) {
    let (mut suite, mut accounts, codes, contracts, _) = setup_test_naive(Default::default());

    let params = multi::Params {
        members: btree_map! {
            accounts.user1.username.clone() => NonZero::new(1).unwrap(),
            accounts.user2.username.clone() => NonZero::new(1).unwrap(),
            accounts.user3.username.clone() => NonZero::new(1).unwrap(),
        },
        voting_period: NonZero::new(Duration::from_seconds(30)).unwrap(),
        threshold: NonZero::new(2).unwrap(),
        // For the purpose of this test, the multisig doesn't have a timelock.
        timelock: None,
    };

    suite
        .execute(
            &mut accounts.user1,
            contracts.account_factory,
            &account_factory::ExecuteMsg::RegisterAccount {
                params: AccountParams::Multi(params.clone()),
            },
            // Fund the multisig with some tokens.
            // The multisig will pay for gas fees, so it must have sufficient tokens.
            Coins::one(usdc::DENOM.clone(), 5_000_000).unwrap(),
        )
        .should_succeed();

    let multi = Multi::new(Addr::derive(
        contracts.account_factory,
        codes.account_multi.to_bytes().hash256(),
        Salt {
            // We have GENESIS_USER_COUNT genesis users, indexed from 0, so the multisig's index
            // should be GENESIS_USER_COUNT.
            index: GENESIS_USER_COUNT,
        }
        .into_bytes()
        .as_slice(),
    ));

    (suite, accounts, contracts, multi, params)
}

#[test]
fn multi_creation() {
    let (suite, accounts, contracts, multi, params) = setup_multi_test();

    // Query the account params.
    suite
        .query_wasm_smart(contracts.account_factory, QueryAccountRequest {
            address: multi.address(),
        })
        .should_succeed_and_equal(Account {
            index: GENESIS_USER_COUNT,
            params: AccountParams::Multi(params.clone()),
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
                // well as the multisig.
                member.address() => Account {
                    index,
                    params: AccountParams::Spot(single::Params::new(
                        member.username.clone()
                    )),
                },
                multi.address() => Account {
                    index: GENESIS_USER_COUNT,
                    params: AccountParams::Multi(params.clone()),
                },
            });
    }

    // The multisig should have received tokens.
    suite
        .query_balance(&multi, usdc::DENOM.clone())
        .should_succeed_and_equal(Uint128::new(5_000_000));
}

#[test]
fn proposal_passing_with_auto_execution() {
    let (mut suite, accounts, _, mut multi, _) = setup_multi_test();
    let multi_address = multi.address();

    // Member 1 makes a proposal to transfer some tokens.
    suite
        .execute(
            multi.with_signer(&accounts.user1),
            multi_address,
            &multi::ExecuteMsg::Propose {
                title: "send 123 uusdc to owner".to_string(),
                description: None,
                messages: vec![
                    Message::transfer(
                        accounts.owner.address(),
                        Coins::one(usdc::DENOM.clone(), 888_888).unwrap(),
                    )
                    .unwrap(),
                ],
            },
            Coins::new(),
        )
        .should_succeed();

    // Member 2 votes YES.
    suite
        .execute(
            multi.with_signer(&accounts.user2),
            multi_address,
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
            multi.with_signer(&accounts.user3),
            multi_address,
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
        .query_wasm_smart(multi.address(), QueryProposalRequest { proposal_id: 1 })
        .should_succeed_and(|prop| prop.status == Status::Executed);

    // Ensure the tokens have been delivered.
    // Owner has 100_000_000_000 uusd to start, and now has received 888_888.
    suite
        .query_balance(&accounts.owner, usdc::DENOM.clone())
        .should_succeed_and_equal(Uint128::new(100_000_888_888));
}

#[test]
fn proposal_passing_with_manual_execution() {
    let (mut suite, accounts, contracts, mut multi, mut params) = setup_multi_test();
    let multi_address = multi.address();

    // Member 1 makes a proposal to amend the members set,
    // adding a new member (`user4`) and removing one (`user3`).
    let updates = multi::ParamUpdates {
        members: ChangeSet::new_unchecked(
            btree_map! {
                accounts.user4.username.clone() => NonZero::new(1).unwrap(),
            },
            btree_set! {
                accounts.user3.username.clone(),
            },
        ),
        voting_period: None,
        threshold: None,
    };

    suite
        .execute(
            multi.with_signer(&accounts.user1),
            multi_address,
            &multi::ExecuteMsg::Propose {
                title: "add user4 as member".to_string(),
                description: None,
                messages: vec![
                    Message::execute(
                        contracts.account_factory,
                        &account_factory::ExecuteMsg::UpdateAccount(AccountParamUpdates::Multi(
                            updates.clone(),
                        )),
                        Coins::new(),
                    )
                    .unwrap(),
                ],
            },
            Coins::new(),
        )
        .should_succeed();

    // Members 2 and 3 votes on the proposal (without auto-execute).
    for member in [&accounts.user2, &accounts.user3] {
        suite
            .execute(
                multi.with_signer(member),
                multi_address,
                &multi::ExecuteMsg::Vote {
                    proposal_id: 1,
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
        .query_wasm_smart(multi.address(), QueryProposalRequest { proposal_id: 1 })
        .should_succeed_and(|prop| matches!(prop.status, Status::Passed { .. }));

    // Member 1 executes the proposal.
    suite
        .execute(
            multi.with_signer(&accounts.user1),
            multi_address,
            &multi::ExecuteMsg::Execute { proposal_id: 1 },
            Coins::new(),
        )
        .should_succeed();

    // Proposal should now be in the "executed" state.
    suite
        .query_wasm_smart(multi.address(), QueryProposalRequest { proposal_id: 1 })
        .should_succeed_and(|prop| prop.status == Status::Executed);

    // Ensure the params have been amended.
    params.apply_updates(updates);

    suite
        .query_wasm_smart(contracts.account_factory, QueryAccountRequest {
            address: multi.address(),
        })
        .should_succeed_and_equal(Account {
            index: GENESIS_USER_COUNT,
            params: AccountParams::Multi(params),
        });

    // The new member
    suite
        .query_wasm_smart(contracts.account_factory, QueryAccountsByUserRequest {
            username: accounts.user4.username.clone(),
        })
        .should_succeed_and(|accounts| accounts.contains_key(&multi.address()));

    // The removed member
    suite
        .query_wasm_smart(contracts.account_factory, QueryAccountsByUserRequest {
            username: accounts.user3.username.clone(),
        })
        .should_succeed_and(|accounts| !accounts.contains_key(&multi.address()));
}

#[test]
fn proposal_failing() {
    let (mut suite, accounts, _, mut multi, _) = setup_multi_test();
    let multi_address = multi.address();

    // Member 1 makes a proposal.
    suite
        .execute(
            multi.with_signer(&accounts.user1),
            multi_address,
            &multi::ExecuteMsg::Propose {
                title: "nothing".to_string(),
                description: None,
                messages: vec![],
            },
            Coins::new(),
        )
        .should_succeed();

    // Member 2 and 3 vote against it.
    for member in [&accounts.user2, &accounts.user3] {
        suite
            .execute(
                multi.with_signer(member),
                multi_address,
                &multi::ExecuteMsg::Vote {
                    proposal_id: 1,
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
        .query_wasm_smart(multi.address(), QueryProposalRequest { proposal_id: 1 })
        .should_succeed_and(|prop| prop.status == Status::Failed);

    // Attempting to execute the proposal should fail.
    suite
        .send_message(
            multi.with_signer(&accounts.user1),
            Message::execute(
                multi_address,
                &multi::ExecuteMsg::Execute { proposal_id: 1 },
                Coins::new(),
            )
            .unwrap(),
        )
        .should_fail_with_error("proposal isn't passed or timelock hasn't elapsed");
}

/// There are 3 cases of unauthorized voting:
///
/// 1. A non-member attempts to vote, impersonating a member.
/// 2. A member attempts to vote, imperonating another member.
/// 3. A user who is currently a member attempts to vote in a proposal that was
///    created at a time when this user wasn't a member.
///
/// This test tests #1.
#[test]
fn unauthorized_voting_via_impersonation_by_a_non_member() {
    let (mut suite, accounts, _, mut multi, _) = setup_multi_test();
    let multi_address = multi.address();

    // A member creates a proposal.
    suite
        .execute(
            multi.with_signer(&accounts.user1),
            multi_address,
            &multi::ExecuteMsg::Propose {
                title: "nothing".to_string(),
                description: None,
                messages: vec![],
            },
            Coins::new(),
        )
        .should_succeed();

    // `user4`, who is not a member, attempts to vote by impersonating `user1`.
    //
    // Since attacker doesn't actual know the member's private key, the tx will
    // be signed by accounts.user4's private key.
    //
    // There are a few variables to consider:
    //
    // - the `voter` field in `ExecuteMsg::Vote`
    // - the `username` field in the metadata
    // - the `key_hash` field in the metadata
    //
    // We test all 2**3 = 8 combinations.
    for (voter, username, key_hash, error) in [
        // First, in `dango_account_multi::authenticate`, the contract checks the
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
        // checks the multisig is associated with the voter's username. That is,
        // the voter is a member of the multi.
        (
            accounts.user4.username.clone(),
            accounts.user4.username.clone(),
            accounts.user4.first_key_hash(),
            format!(
                "voter `{}` is not eligible to vote in this proposal",
                accounts.user4.username
            ),
        ),
        (
            accounts.user4.username.clone(),
            accounts.user4.username.clone(),
            accounts.user2.first_key_hash(),
            format!(
                "voter `{}` is not eligible to vote in this proposal",
                accounts.user4.username
            ),
        ),
        // Now we know the voter and username must both be that of a member.
        (
            accounts.user2.username.clone(),
            accounts.user2.username.clone(),
            accounts.user4.first_key_hash(),
            {
                let path = KEYS.path((&accounts.user2.username, accounts.user4.first_key_hash()));
                StdError::data_not_found::<Key>(path.storage_key()).to_string()
            },
        ),
        (
            accounts.user2.username.clone(),
            accounts.user2.username.clone(),
            accounts.user2.first_key_hash(),
            "signature is unauthentic".to_string(),
        ),
    ] {
        unauthorized_voting_via_impersonation(
            &mut suite,
            multi.with_nonce(1), /* TODO: nonce isn't incremented if auth fails... should we make sure it increments? */
            &accounts.user4,
            voter,
            username,
            key_hash,
            error,
        );
    }
}

/// This tests the scenario #2 in authorized voting.
#[test]
fn unauthorized_voting_via_impersonation_by_a_member() {
    let (mut suite, accounts, _, mut multi, _) = setup_multi_test();
    let multi_address = multi.address();

    // A member creates a proposal.
    suite
        .execute(
            multi.with_signer(&accounts.user1),
            multi_address,
            &multi::ExecuteMsg::Propose {
                title: "nothing".to_string(),
                description: None,
                messages: vec![],
            },
            Coins::new(),
        )
        .should_succeed();

    // The attacker (`user3`) votes first.
    suite
        .execute(
            multi.with_signer(&accounts.user3),
            multi_address,
            &multi::ExecuteMsg::Vote {
                proposal_id: 1,
                voter: accounts.user3.username.clone(),
                vote: Vote::Yes,
                execute: false,
            },
            Coins::new(),
        )
        .should_succeed();

    // `user3`, who is a member but already voted, attempts to vote again by
    // impersonating `user1`.
    for (voter, username, key_hash, nonce, error) in [
        (
            accounts.user3.username.clone(),
            accounts.user2.username.clone(),
            accounts.user3.first_key_hash(),
            2,
            "can't vote with a different username".to_string(),
        ),
        (
            accounts.user3.username.clone(),
            accounts.user2.username.clone(),
            accounts.user2.first_key_hash(),
            2,
            "can't vote with a different username".to_string(),
        ),
        (
            accounts.user2.username.clone(),
            accounts.user3.username.clone(),
            accounts.user3.first_key_hash(),
            2,
            "can't vote with a different username".to_string(),
        ),
        (
            accounts.user2.username.clone(),
            accounts.user3.username.clone(),
            accounts.user2.first_key_hash(),
            2,
            "can't vote with a different username".to_string(),
        ),
        (
            accounts.user3.username.clone(),
            accounts.user3.username.clone(),
            accounts.user3.first_key_hash(),
            2,
            format!(
                "user `{}` has already voted in this proposal",
                accounts.user3.username
            ),
        ),
        // The previous test passes `authenticate`, but fails in `execute`, so
        // the nonce should be incremented.
        (
            accounts.user3.username.clone(),
            accounts.user3.username.clone(),
            accounts.user2.first_key_hash(),
            3,
            {
                let path = KEYS.path((&accounts.user3.username, accounts.user2.first_key_hash()));
                StdError::data_not_found::<Key>(path.storage_key()).to_string()
            },
        ),
        (
            accounts.user2.username.clone(),
            accounts.user2.username.clone(),
            accounts.user3.first_key_hash(),
            3,
            {
                let path = KEYS.path((&accounts.user2.username, accounts.user3.first_key_hash()));
                StdError::data_not_found::<Key>(path.storage_key()).to_string()
            },
        ),
        (
            accounts.user2.username.clone(),
            accounts.user2.username.clone(),
            accounts.user2.first_key_hash(),
            3,
            "signature is unauthentic".to_string(),
        ),
    ] {
        unauthorized_voting_via_impersonation(
            &mut suite,
            multi.with_nonce(nonce), /* TODO: nonce isn't incremented if auth fails... should we make sure it increment? */
            &accounts.user3,
            voter,
            username,
            key_hash,
            error,
        );
    }
}

fn unauthorized_voting_via_impersonation<'a>(
    suite: &mut TestSuite<NaiveProposalPreparer>,
    multi: &mut Multi<'a>,
    // An attacker who attempts to illegally vote by impersonating a member.
    attacker: &'a TestAccount,
    // The voter usrname that the attacker will put in the `ExecuteMsg::Vote`.
    voter: Username,
    // The username that the attacker will put in the metadata.
    username: Username,
    // The key hash that the attacker will put in the metadata.
    key_hash: Hash256,
    // The expected error
    error: String,
) {
    let multi_address = multi.address();

    // Sign the tx with attacker's private key.
    let mut tx = multi
        .with_signer(attacker)
        .sign_transaction(
            NonEmpty::new_unchecked(vec![
                Message::execute(
                    multi_address,
                    &multi::ExecuteMsg::Vote {
                        proposal_id: 1,
                        voter,
                        vote: Vote::Yes,
                        execute: false,
                    },
                    Coins::new(),
                )
                .unwrap(),
            ]),
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

/// Any action a multisig account does must be though a passed proposal.
/// Attempting otherwise should be rejected.
#[test]
fn unauthorized_messages() {
    let (mut suite, accounts, contracts, mut multi, _) = setup_multi_test();

    // Attempt to send a `MsgTransfer` from the multisig without a proposal.
    // Should fail.
    suite
        .transfer(
            multi.with_signer(&accounts.user1),
            accounts.user1.address(),
            Coins::one(usdc::DENOM.clone(), 123).unwrap(),
        )
        .should_fail_with_error("illegal action for a multi-signature account");

    // Attempt to send a `MsgExecute` from the multisig where the contract being
    // executed is not the multisig itself. Should fail with the same error.
    suite
        .execute(
            multi.with_signer(&accounts.user1),
            contracts.lending,
            &Empty {}, // the message doesn't matter
            Coins::new(),
        )
        .should_fail_with_error("illegal action for a multi-signature account");
}

/// When creating, voting for, or executing a proposal, the member must use the
/// multisig account has the transaction's `sender`.
#[test]
fn unauthorized_execute() {
    let (mut suite, mut accounts, _, multi, _) = setup_multi_test();

    suite
        .execute(
            &mut accounts.user1,
            multi.address(),
            &multi::ExecuteMsg::Propose {
                title: "nothing".to_string(),
                description: None,
                messages: vec![],
            },
            Coins::new(),
        )
        .should_fail_with_error("only the multisig account itself can execute itself");
}

/// Whether someone can vote in a proposal is determined by whether they were a
/// member _at the time the proposal was created_. This leads to two edge cases:
///
/// ## Edge case 1
///
/// - A proposal is created when the user is a member.
/// - Another proposal passes to remove the user's membership.
/// - The user attempts to vote.
///
/// This transaction should be ACCEPTED, despite the user is NOT a a current
/// member.
///
/// ## Edge case 2
///
/// - A proposal is created when the user is NOT a member.
/// - Another proposal passes to add the user as a member.
/// - The user attempts to vote.
///
/// This transaction should be REJECT, despite the user IS a current member.
#[test]
fn vote_edge_cases() {
    let (mut suite, accounts, contracts, mut multi, _) = setup_multi_test();
    let multi_address = multi.address();

    // Member 1:
    // - makes a proposal;
    // - makes another proposal to remove `user3` and add `user4`;
    // - vote in the second proposal.
    //
    // Member 2:
    // - vote in the second proposal with auto-execute.
    suite
        .execute(
            multi.with_signer(&accounts.user1),
            multi_address,
            &multi::ExecuteMsg::Propose {
                title: "nothing".to_string(),
                description: None,
                messages: vec![],
            },
            Coins::new(),
        )
        .should_succeed();

    suite
        .execute(
            multi.with_signer(&accounts.user1),
            multi_address,
            &multi::ExecuteMsg::Propose {
                title: "remove user3".to_string(),
                description: None,
                messages: vec![Message::execute(
                    contracts.account_factory,
                    &account_factory::ExecuteMsg::UpdateAccount(AccountParamUpdates::Multi(
                        multi::ParamUpdates {
                            members: ChangeSet::new_unchecked(
                                btree_map! {
                                    accounts.user4.username.clone() => NonZero::new_unchecked(1),
                                },
                                btree_set! {
                                    accounts.user3.username.clone(),
                                },
                            ),
                            threshold: None,
                            voting_period: None,
                        },
                    )),
                    Coins::new(),
                )
                .unwrap()],
            },
            Coins::new(),
        )
        .should_succeed();

    suite
        .execute(
            multi.with_signer(&accounts.user1),
            multi_address,
            &multi::ExecuteMsg::Vote {
                proposal_id: 2,
                voter: accounts.user1.username.clone(),
                vote: Vote::Yes,
                execute: false,
            },
            Coins::new(),
        )
        .should_succeed();

    suite
        .execute(
            multi.with_signer(&accounts.user2),
            multi_address,
            &multi::ExecuteMsg::Vote {
                proposal_id: 2,
                voter: accounts.user2.username.clone(),
                vote: Vote::Yes,
                execute: true,
            },
            Coins::new(),
        )
        .should_succeed();

    // Now, `user3` should no longer be a member, while `user4` should be one.
    suite
        .query_wasm_smart(contracts.account_factory, QueryAccountRequest {
            address: multi_address,
        })
        .should_succeed_and(|account| {
            let members = account.params.clone().as_multi().members;
            !members.contains_key(&accounts.user3.username)
                && members.contains_key(&accounts.user4.username)
        });

    // `user3` attempts to vote in first proposal. Should be accepted!
    suite
        .execute(
            multi.with_signer(&accounts.user3),
            multi_address,
            &multi::ExecuteMsg::Vote {
                proposal_id: 1,
                voter: accounts.user3.username.clone(),
                vote: Vote::Yes,
                execute: true,
            },
            Coins::new(),
        )
        .should_succeed();

    // `user3`'s vote should have been recorded.
    suite
        .query_wasm_smart(multi_address, QueryVoteRequest {
            proposal_id: 1,
            member: accounts.user3.username.clone(),
        })
        .should_succeed_and_equal(Some(Vote::Yes));

    // `user4` attempts to vote in the first proposal. Should be rejected!
    suite
        .execute(
            multi.with_signer(&accounts.user4),
            multi_address,
            &multi::ExecuteMsg::Vote {
                proposal_id: 1,
                voter: accounts.user4.username.clone(),
                vote: Vote::Yes,
                execute: true,
            },
            Coins::new(),
        )
        .should_fail_with_error(format!(
            "voter `{}` is not eligible to vote in this proposal",
            accounts.user4.username
        ));

    // `user4`'s vote should NOT have been recorded.
    suite
        .query_wasm_smart(multi_address, QueryVoteRequest {
            proposal_id: 1,
            member: accounts.user4.username.clone(),
        })
        .should_succeed_and_equal(None);
}

#[test]
fn non_member_cannot_create_proposal() {
    let (mut suite, accounts, _, mut multi, _) = setup_multi_test();
    let multi_address = multi.address();

    suite
        .execute(
            multi.with_signer(&accounts.user4), // not a member
            multi_address,
            &multi::ExecuteMsg::Propose {
                title: "nothing".to_string(),
                description: None,
                messages: vec![],
            },
            Coins::new(),
        )
        .should_fail_with_error(format!(
            "account {} isn't associated with user `{}`",
            multi_address, accounts.user4.username
        ));
}

/// Test whether the auth implementation can deter a specific DoS attack
/// involving nonces.
///
/// See the comments of `dango_auth::MAX_NONCE_INCREASE` for more details.
#[test]
fn max_nonce_dos_attack() {
    let (mut suite, accounts, _, mut multi, _) = setup_multi_test();
    let multi_address = multi.address();

    // Member 1 makes a proposal. This should come with nonce 0.
    suite
        .execute(
            multi.with_signer(&accounts.user1),
            multi_address,
            &multi::ExecuteMsg::Propose {
                title: "empty proposal".to_string(),
                description: None,
                messages: vec![],
            },
            Coins::new(),
        )
        .should_succeed();

    // Member 2 attempts to vote, with a nonce that "jumps", but not too much
    // bigger than the biggest seen nonce. This should work.
    {
        let msgs = NonEmpty::new_unchecked(vec![
            Message::execute(
                multi_address,
                &multi::ExecuteMsg::Vote {
                    proposal_id: 1,
                    voter: accounts.user2.username.clone(),
                    vote: Vote::Yes,
                    execute: false,
                },
                Coins::new(),
            )
            .unwrap(),
        ]);

        let (metadata, credential) = accounts
            .user2
            .sign_transaction_with_nonce(
                multi_address,
                msgs.clone(),
                &suite.chain_id,
                100_000,
                5, // should be 1, but we jump to 5
                None,
            )
            .unwrap();

        let tx = Tx {
            sender: multi_address,
            gas_limit: 100_000,
            msgs,
            data: metadata.to_json_value().unwrap(),
            credential: credential.to_json_value().unwrap(),
        };

        suite.send_transaction(tx).should_succeed();
    }

    // Member 3 (bad guy) attempts to DoS attack. He attempts to vote with a
    // very big nonce. Should fail. Specifically, it should fail at the `CheckTx`
    // stage, meaning the tx won't enter the mempool at all.
    {
        let msgs = NonEmpty::new_unchecked(vec![
            Message::execute(
                multi_address,
                &multi::ExecuteMsg::Vote {
                    proposal_id: 1,
                    voter: accounts.user3.username.clone(),
                    vote: Vote::Yes,
                    execute: false,
                },
                Coins::new(),
            )
            .unwrap(),
        ]);

        let (metadata, credential) = accounts
            .user3
            .sign_transaction_with_nonce(
                multi_address,
                msgs.clone(),
                &suite.chain_id,
                100_000,
                123456, // maximum allowed is 5 + 100 = 105
                None,
            )
            .unwrap();

        let tx = Tx {
            sender: multi_address,
            gas_limit: 100_000,
            msgs,
            data: metadata.to_json_value().unwrap(),
            credential: credential.to_json_value().unwrap(),
        };

        suite.check_tx(tx).should_fail_with_error(format!(
            "nonce is too far ahead: {} > {} + MAX_NONCE_INCREASE ({})",
            123456, 5, MAX_NONCE_INCREASE
        ));
    }
}
