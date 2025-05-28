use {
    corepc_client::bitcoin::Network,
    dango_testing::{TestAccount, TestAccounts, TestSuite, setup_test_naive},
    dango_types::{
        bitcoin::{
            Config, ExecuteMsg, InboundConfirmed, InstantiateMsg, QueryConfigRequest,
            QueryOutboundQueueRequest, QueryOutboundTransactionRequest, QueryUtxosRequest,
        },
        constants::btc,
        gateway::{
            self,
            bridge::{BridgeMsg, TransferRemoteRequest},
        },
    },
    grug::{
        Addr, CheckedContractEvent, Coins, Duration, Hash256, HashExt, JsonDeExt, Message,
        NonEmpty, Order, QuerierExt, ResultExt, SearchEvent, Uint128, btree_map, btree_set, coins,
    },
    grug_app::NaiveProposalPreparer,
    std::str::FromStr,
};

// Create and confirm a deposit to bitcoin bridge contract.
fn deposit(
    suite: &mut TestSuite<NaiveProposalPreparer>,
    bitcoin_contract: Addr,
    accounts: &mut TestAccounts,
    amount: Uint128,
    recipient: Option<Addr>,
    index: u64,
) {
    let mut bytes = [0u8; 32];
    bytes[24..].copy_from_slice(&index.to_le_bytes());

    let msg = ExecuteMsg::ObserveInbound {
        transaction_hash: Hash256::from_inner(bytes),
        vout: 1,
        amount,
        recipient,
    };

    let msg = Message::execute(bitcoin_contract, &msg, Coins::new()).unwrap();

    // Needs 2/3 guardians to confirm the deposit.
    suite
        .send_message(&mut accounts.val1, msg.clone())
        .should_succeed();

    suite
        .send_message(&mut accounts.val2, msg.clone())
        .should_succeed();
}

fn withdraw(
    suite: &mut TestSuite<NaiveProposalPreparer>,
    user: &mut TestAccount,
    gateway_contract: Addr,
    amount: Uint128,
    recipient: &str,
) {
    let msg = gateway::ExecuteMsg::TransferRemote(TransferRemoteRequest::Bitcoin {
        recipient: recipient.to_string(),
    });

    suite
        .execute(
            user,
            gateway_contract,
            &msg,
            coins! { btc::DENOM.clone() => amount },
        )
        .should_succeed();
}

// Advance 10 minutes in the test suite, which is enough for the cron job to execute.
fn advance_ten_minutes(suite: &mut TestSuite<NaiveProposalPreparer>) {
    suite.block_time = Duration::from_minutes(10);
    let b = suite.make_empty_block();
    for cron_outcom in b.block_outcome.cron_outcomes {
        if let Some(error) = cron_outcom.cron_event.maybe_error() {
            panic!("cron job failed: {error}");
        }
    }
    suite.block_time = Duration::ZERO;
}

#[test]
fn instantiate() {
    let (mut suite, accounts, codes, ..) = setup_test_naive(Default::default());

    let mut owner = accounts.owner;
    let owner_address = owner.address.inner().clone();
    let bitcoin_hash = codes.bitcoin.to_bytes().hash256();

    // Try to instantiate the contract with wrong address.
    {
        let config = Config {
            network: Network::Bitcoin,
            vault: "Hello Dango!".to_string(),
            guardians: NonEmpty::new_unchecked(btree_set!(
                accounts.user1.address.inner().clone(),
                accounts.user2.address.inner().clone(),
                accounts.user3.address.inner().clone(),
            )),
            threshold: 2,
            sats_per_vbyte: Uint128::new(10),
            outbound_strategy: Order::Ascending,
            minimum_deposit: Uint128::new(1000),
        };

        suite
            .instantiate(
                &mut owner,
                bitcoin_hash,
                &InstantiateMsg { config },
                "salt",
                Some("bitcoin_bridge_test"),
                Some(owner_address),
                Coins::new(),
            )
            .should_fail_with_error("is not a valid Bitcoin address");
    }

    // Try to instantiate the contract with wrong combination:
    // - Network::Testnet
    // - vault address a valid bitcoin mainnet address.
    {
        let config = Config {
            network: Network::Testnet,
            vault: "1PuJjnF476W3zXfVYmJfGnouzFDAXakkL4".to_string(),
            guardians: NonEmpty::new_unchecked(btree_set!(
                accounts.user1.address.inner().clone(),
                accounts.user2.address.inner().clone(),
                accounts.user3.address.inner().clone(),
            )),
            threshold: 2,
            sats_per_vbyte: Uint128::new(10),
            outbound_strategy: Order::Ascending,
            minimum_deposit: Uint128::new(1000),
        };

        suite
            .instantiate(
                &mut owner,
                bitcoin_hash,
                &InstantiateMsg { config },
                "salt",
                Some("bitcoin_bridge"),
                Some(owner_address),
                Coins::new(),
            )
            .should_fail_with_error("is not a valid Bitcoin address for network");
    }

    // Try to instantiate the contract with right combination.
    {
        let config = Config {
            network: Network::Bitcoin,
            vault: "1PuJjnF476W3zXfVYmJfGnouzFDAXakkL4".to_string(),
            guardians: NonEmpty::new_unchecked(btree_set!(
                accounts.user1.address.inner().clone(),
                accounts.user2.address.inner().clone(),
                accounts.user3.address.inner().clone(),
            )),
            threshold: 2,
            sats_per_vbyte: Uint128::new(10),
            outbound_strategy: Order::Ascending,
            minimum_deposit: Uint128::new(1000),
        };

        suite
            .instantiate(
                &mut owner,
                bitcoin_hash,
                &InstantiateMsg { config },
                "salt",
                Some("bitcoin_bridge"),
                Some(owner_address),
                Coins::new(),
            )
            .should_succeed();
    }
}

#[test]
fn observe_inbound() {
    let (mut suite, mut accounts, _, contracts, ..) = setup_test_naive(Default::default());

    // Report a deposit with an amount lower than min deposit.
    let msg = ExecuteMsg::ObserveInbound {
        transaction_hash: Hash256::from_inner([0; 32]),
        vout: 1,
        amount: Uint128::new(100),
        recipient: None,
    };

    suite
        .execute(&mut accounts.val1, contracts.bitcoin, &msg, Coins::new())
        .should_fail_with_error("minimum deposit not met");

    // Report a deposit.
    let bitcoin_tx_hash =
        Hash256::from_str("C42F8B7FEFBDDE209F16A3084D9A5B44913030322F3AF27459A980674A7B9356")
            .unwrap();
    let vout = 1;
    let amount = Uint128::new(2000);
    let recipient = accounts.user1.address.inner().clone();

    let msg = ExecuteMsg::ObserveInbound {
        transaction_hash: bitcoin_tx_hash,
        vout,
        amount,
        recipient: Some(recipient),
    };

    let msg = Message::execute(contracts.bitcoin, &msg, Coins::new()).unwrap();

    // Broadcast the message with a non guardian signer.
    suite
        .send_message(&mut accounts.user4, msg.clone())
        .should_fail_with_error("you don't have the right, O you don't have the right");

    // Broadcast the message with first guardian signer.
    suite
        .send_message(&mut accounts.val1, msg.clone())
        .should_succeed();

    // Broadcast again the message with the same signer (should fail).
    suite
        .send_message(&mut accounts.val1, msg.clone())
        .should_fail_with_error("you've already voted for transaction");

    // Broadcast the message with second guardian signer.
    // The threshold is met so there should be the event.
    suite
        .send_message(&mut accounts.val2, msg.clone())
        .should_succeed()
        .events
        .search_event::<CheckedContractEvent>()
        .with_predicate(|evt| evt.ty == "inbound_confirmed")
        .take()
        .one()
        .event
        .data
        .deserialize_json::<InboundConfirmed>()
        .should_succeed_and_equal(InboundConfirmed {
            transaction_hash: bitcoin_tx_hash,
            amount,
            recipient: Some(recipient),
        });

    let balance = suite.query_balance(&recipient, btc::DENOM.clone()).unwrap();
    assert_eq!(
        balance, amount,
        "recipient has wrong btc balance! expecting: {amount}, found: {balance}",
    );

    // Broadcast the message with third guardian signer
    // (should fail since already match the threshold).
    suite
        .send_message(&mut accounts.val3, msg.clone())
        .should_fail_with_error("already exists in UTXO set");

    // Ensure the inbound works with None recipient.
    {
        let tx_hash =
            Hash256::from_str("14A0BF02F69BD13C274ED22E20C1BF4CC5DABF99753DB32E5B8959BF4C5F1F5C")
                .unwrap();
        let msg = ExecuteMsg::ObserveInbound {
            transaction_hash: tx_hash,
            vout: 2,
            amount,
            recipient: None,
        };

        let msg = Message::execute(contracts.bitcoin, &msg, Coins::new()).unwrap();

        // Broadcast the message with a non guardian signer.
        suite
            .send_message(&mut accounts.val1, msg.clone())
            .should_succeed();

        // Broadcast the message with a non guardian signer.
        suite
            .send_message(&mut accounts.val2, msg.clone())
            .should_succeed()
            .events
            .search_event::<CheckedContractEvent>()
            .with_predicate(|evt| evt.ty == "inbound_confirmed")
            .take()
            .one()
            .event
            .data
            .deserialize_json::<InboundConfirmed>()
            .should_succeed_and_equal(InboundConfirmed {
                transaction_hash: tx_hash,
                amount,
                recipient: None,
            });
    }
}

#[test]
fn transfer_remote() {
    let (mut suite, mut accounts, _, contracts, ..) = setup_test_naive(Default::default());

    let btc_recipient = "bcrt1q8qzecux6rz9aatnpjulmfrraznyqjc3crq33m0".to_string();
    let user1_address = accounts.user1.address.inner().clone();

    // Deposit 100k sats do user1
    deposit(
        &mut suite,
        contracts.bitcoin,
        &mut accounts,
        Uint128::new(100_000),
        Some(user1_address),
        0,
    );

    // Interact directly to the bride (only gateway can).
    {
        let msg = ExecuteMsg::Bridge(BridgeMsg::TransferRemote {
            req: TransferRemoteRequest::Bitcoin {
                recipient: btc_recipient.clone(),
            },
            amount: Uint128::new(100),
        });

        suite
            .execute(&mut accounts.user1, contracts.bitcoin, &msg, Coins::new())
            .should_fail_with_error("only gateway can call `transfer_remote`");
    }

    // Ensure the btc recipient is checked.
    {
        let msg = gateway::ExecuteMsg::TransferRemote(TransferRemoteRequest::Bitcoin {
            recipient: "invalid_bitcoin_address".to_string(),
        });

        suite
            .execute(
                &mut accounts.user1,
                contracts.gateway,
                &msg,
                coins! { btc::DENOM.clone() => 10_000 },
            )
            .should_fail_with_error("is not a valid Bitcoin address");

        let msg = gateway::ExecuteMsg::TransferRemote(TransferRemoteRequest::Bitcoin {
            recipient: "1PuJjnF476W3zXfVYmJfGnouzFDAXakkL4".to_string(),
        });

        suite
            .execute(
                &mut accounts.user1,
                contracts.gateway,
                &msg,
                coins! { btc::DENOM.clone() => 10_000 },
            )
            .should_fail_with_error("is not a valid Bitcoin address for network");
    }

    // Retrieve the withdrawal fee from the gateway contract.
    let withdraw_fee = suite
        .query_wasm_smart(contracts.gateway, gateway::QueryWithdrawalFeeRequest {
            denom: btc::DENOM.clone(),
            remote: gateway::Remote::Bitcoin,
        })
        .unwrap()
        .unwrap();

    // Create a correct withdrawal.
    let withdraw_amount1 = Uint128::new(10_000);
    {
        let msg = gateway::ExecuteMsg::TransferRemote(TransferRemoteRequest::Bitcoin {
            recipient: btc_recipient.clone(),
        });

        suite
            .execute(
                &mut accounts.user1,
                contracts.gateway,
                &msg,
                coins! { btc::DENOM.clone() => withdraw_amount1 },
            )
            .should_succeed();

        // Ensure the data is stored in the contract.
        suite
            .query_wasm_smart(contracts.bitcoin, QueryOutboundQueueRequest {
                start_after: None,
                limit: None,
            })
            .should_succeed_and_equal(btree_map!(
                btc_recipient.clone() => withdraw_amount1 - withdraw_fee
            ));
    }

    // Ensure that, If an user start a second withdrawal, the withdrawals are combined in one.
    let withdraw_amount2 = Uint128::new(20_000);
    {
        let withdraw_fee = suite
            .query_wasm_smart(contracts.gateway, gateway::QueryWithdrawalFeeRequest {
                denom: btc::DENOM.clone(),
                remote: gateway::Remote::Bitcoin,
            })
            .unwrap()
            .unwrap();

        let msg = gateway::ExecuteMsg::TransferRemote(TransferRemoteRequest::Bitcoin {
            recipient: btc_recipient.clone(),
        });

        suite
            .execute(
                &mut accounts.user1,
                contracts.gateway,
                &msg,
                coins! { btc::DENOM.clone() => withdraw_amount2 },
            )
            .should_succeed();

        // Ensure the data is stored in the contract.
        suite
            .query_wasm_smart(contracts.bitcoin, QueryOutboundQueueRequest {
                start_after: None,
                limit: None,
            })
            .should_succeed_and_equal(btree_map!(
                btc_recipient.clone() => withdraw_amount1 + withdraw_amount2 - withdraw_fee - withdraw_fee
            ));
    }

    // Adding a withdrawal with a different recipient.
    {
        let withdraw_amount3 = Uint128::new(30_000);
        let recipient2 = "bcrt1q4e3mwznnr3chnytav5h4mhx52u447jv2kl55z9".to_string();

        let msg = gateway::ExecuteMsg::TransferRemote(TransferRemoteRequest::Bitcoin {
            recipient: recipient2.clone(),
        });

        suite
            .execute(
                &mut accounts.user1,
                contracts.gateway,
                &msg,
                coins! { btc::DENOM.clone() => withdraw_amount3 },
            )
            .should_succeed();

        // Ensure the data is stored in the contract.
        suite
            .query_wasm_smart(contracts.bitcoin, QueryOutboundQueueRequest {
                start_after: None,
                limit: None,
            })
            .should_succeed_and_equal(btree_map!(
                btc_recipient.clone() => withdraw_amount1 + withdraw_amount2 - withdraw_fee - withdraw_fee,
                recipient2.clone() => withdraw_amount3 - withdraw_fee
            ));
    }
}

#[test]
fn cron_execute() {
    let (mut suite, mut accounts, _, contracts, ..) = setup_test_naive(Default::default());

    suite.block_time = Duration::ZERO;

    let vault = suite
        .query_wasm_smart(contracts.bitcoin, QueryConfigRequest {})
        .unwrap()
        .vault;

    let user1_address = accounts.user1.address.inner().clone();

    let withdraw_fee = suite
        .query_wasm_smart(contracts.gateway, gateway::QueryWithdrawalFeeRequest {
            denom: btc::DENOM.clone(),
            remote: gateway::Remote::Bitcoin,
        })
        .unwrap()
        .unwrap();

    // Deposit 100k sats do user1
    deposit(
        &mut suite,
        contracts.bitcoin,
        &mut accounts,
        Uint128::new(100_000),
        Some(user1_address),
        0,
    );

    // Make 2 withdrawals.
    let withdraw_amount1 = Uint128::new(10_000);
    let net_withdraw1 = withdraw_amount1 - withdraw_fee;
    let recipient1 = "bcrt1q8qzecux6rz9aatnpjulmfrraznyqjc3crq33m0".to_string();

    withdraw(
        &mut suite,
        &mut accounts.user1,
        contracts.gateway,
        withdraw_amount1,
        &recipient1,
    );

    let withdraw_amount2 = Uint128::new(20_000);
    let net_withdraw2 = withdraw_amount2 - withdraw_fee;
    let recipient2 = "bcrt1q4e3mwznnr3chnytav5h4mhx52u447jv2kl55z9".to_string();

    withdraw(
        &mut suite,
        &mut accounts.user1,
        contracts.gateway,
        withdraw_amount2,
        &recipient2,
    );

    // Ensure the data is stored in the contract.
    suite
        .query_wasm_smart(contracts.bitcoin, QueryOutboundQueueRequest {
            start_after: None,
            limit: None,
        })
        .should_succeed_and_equal(btree_map!(
            recipient1.clone() => net_withdraw1,
            recipient2.clone() => net_withdraw2
        ));

    // Wait for the cron job to execute.
    advance_ten_minutes(&mut suite);

    // Ensure the outbound queue is empty.
    suite
        .query_wasm_smart(contracts.bitcoin, QueryOutboundQueueRequest {
            start_after: None,
            limit: None,
        })
        .should_succeed_and_equal(btree_map!());

    // Ensure there is a withdrawal.
    let tx = suite
        .query_wasm_smart(contracts.bitcoin, QueryOutboundTransactionRequest { id: 0 })
        .should_succeed();

    assert_eq!(
        tx.inputs,
        btree_map!( (Hash256::from_inner([0u8; 32]), 1) => Uint128::new(100_000) )
    );

    assert_eq!(
        tx.outputs,
        btree_map!(
            recipient1.clone() => net_withdraw1,
            recipient2.clone() => net_withdraw2,
            vault => Uint128::new(100_000) - net_withdraw1 - net_withdraw2 -tx.fee
        )
    );

    // Ensure the UTXO is no more in the available set.
    suite
        .query_wasm_smart(contracts.bitcoin, QueryUtxosRequest {
            start_after: None,
            limit: None,
            order: Order::Ascending,
        })
        .should_succeed_and_equal(vec![]);
}

#[test]
fn authorize_outbound() {
    let (mut suite, mut accounts, _, contracts, ..) = setup_test_naive(Default::default());

    suite.block_time = Duration::ZERO;

    let user1_address = accounts.user1.address.inner().clone();

    // Deposit 100k sats do user1
    deposit(
        &mut suite,
        contracts.bitcoin,
        &mut accounts,
        Uint128::new(100_000),
        Some(user1_address),
        0,
    );

    // Create a withdrawal.
    withdraw(
        &mut suite,
        &mut accounts.user1,
        contracts.gateway,
        Uint128::new(10_000),
        "bcrt1q4e3mwznnr3chnytav5h4mhx52u447jv2kl55z9",
    );

    advance_ten_minutes(&mut suite);

    // Ensure that only validator can call `authorize_outbound`.
    {
        let msg = ExecuteMsg::AuthorizeOutbound {
            id: 0,
            signatures: vec![],
        };

        suite
            .execute(&mut accounts.user1, contracts.bitcoin, &msg, Coins::new())
            .should_fail_with_error("you don't have the right, O you don't have the right");
    }

    // Ensure there is 1 signature per input.
    {
        let msg = ExecuteMsg::AuthorizeOutbound {
            id: 0,
            signatures: vec![],
        };

        suite
            .execute(&mut accounts.val1, contracts.bitcoin, &msg, Coins::new())
            .should_fail_with_error("transaction `0` has 1 inputs, but 0 signatures were provided");
    }

    // TODO:
    // - check the signer does not sign 2 times
    // - check the signatures are valid
}
