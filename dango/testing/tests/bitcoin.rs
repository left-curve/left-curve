use {
    corepc_client::bitcoin::Network,
    dango_testing::setup_test_naive,
    dango_types::{
        bitcoin::{
            Config, ExecuteMsg, InboundConfirmed, InstantiateMsg, QueryOutboundQueueRequest,
        },
        constants::btc,
        gateway::{
            self,
            bridge::{BridgeMsg, TransferRemoteRequest},
        },
    },
    grug::{
        CheckedContractEvent, Coins, Hash256, HashExt, JsonDeExt, Message, NonEmpty, Order,
        QuerierExt, ResultExt, SearchEvent, Uint128, btree_map, btree_set, coins,
    },
    std::str::FromStr,
};

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

    // Report a deposit.
    let bitcoin_tx_hash =
        Hash256::from_str("C42F8B7FEFBDDE209F16A3084D9A5B44913030322F3AF27459A980674A7B9356")
            .unwrap();
    let vout = 1;
    let amount = Uint128::new(100);
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

    // Deposit 100k sats do user1
    {
        let deposit_amount = Uint128::new(100_000);

        let msg = ExecuteMsg::ObserveInbound {
            transaction_hash: Hash256::from_str(
                "C42F8B7FEFBDDE209F16A3084D9A5B44913030322F3AF27459A980674A7B9356",
            )
            .unwrap(),
            vout: 1,
            amount: deposit_amount,
            recipient: Some(accounts.user1.address.inner().clone()),
        };

        let msg = Message::execute(contracts.bitcoin, &msg, Coins::new()).unwrap();

        // Needs 2/3 guardians to confirm the deposit.
        suite
            .send_message(&mut accounts.val1, msg.clone())
            .should_succeed();

        suite
            .send_message(&mut accounts.val2, msg.clone())
            .should_succeed();
    }

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

    // Create a real withdrawal.
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

// // Try to withdraw with wrong remote.
//     {
//         let msg = gateway::ExecuteMsg::TransferRemote(TransferRemoteRequest::Warp {
//             warp_remote: WarpRemote {
//                 domain: ethereum::DOMAIN,
//                 contract: ethereum::USDC_WARP,
//             },
//             recipient: addr32!("0000000000000000000000000000000000000000000000000000000000000000"),
//         });

//         suite
//             .execute(
//                 &mut accounts.user1,
//                 contracts.gateway,
//                 &msg,
//                 coins! { usdc::DENOM.clone() => 10_000_000 },
//             )
//             .should_fail_with_error(
//                 "incorrect TransferRemoteRequest type! expected: Bitcoin, found",
//             );
//     }
