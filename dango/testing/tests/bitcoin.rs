use {
    corepc_client::bitcoin::{
        Amount, EcdsaSighashType, Network, Script, Transaction as BtcTransaction,
        hashes::Hash,
        key::Secp256k1,
        secp256k1::{Message as BtcMessage, SecretKey},
        sighash::SighashCache,
    },
    dango_testing::{
        MOCK_BITCOIN_REGTEST_VAULT, MOCK_BRIDGE_GUARDIANS_KEYS, TestAccount, TestAccounts,
        TestSuite, setup_test_naive,
    },
    dango_types::{
        bitcoin::{
            BitcoinSignature, Config, ExecuteMsg, InboundConfirmed, InstantiateMsg,
            MultisigSettings, OutboundConfirmed, QueryConfigRequest, QueryOutboundQueueRequest,
            QueryOutboundTransactionRequest, QueryUtxosRequest,
        },
        constants::btc,
        gateway::{
            self,
            bridge::{BridgeMsg, TransferRemoteRequest},
        },
    },
    grug::{
        Addr, CheckedContractEvent, Coins, Duration, Hash256, HashExt, HexBinary, HexByteArray,
        Inner, JsonDeExt, Message, NonEmpty, Order, PrimaryKey, QuerierExt, ResultExt, SearchEvent,
        Uint128, btree_map, btree_set, coins,
    },
    grug_app::NaiveProposalPreparer,
    std::{str::FromStr, vec},
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

pub fn sing_inputs(
    tx: &BtcTransaction,
    sk: &SecretKey,
    redeem_script: &Script,
    amounts: Vec<u64>,
) -> Vec<HexBinary> {
    let mut cache = SighashCache::new(tx);
    amounts
        .into_iter()
        .enumerate()
        .map(|(i, amount)| {
            let sighash = cache
                .p2wsh_signature_hash(
                    i,
                    redeem_script,
                    Amount::from_sat(amount),
                    EcdsaSighashType::All, // To sign all inputs and outputs
                )
                .unwrap();

            let secp = Secp256k1::new();
            let msg = BtcMessage::from_digest(sighash.to_byte_array());
            let sig = secp.sign_ecdsa(&msg, sk);
            let mut der_sig = sig.serialize_der().to_vec();
            der_sig.push(EcdsaSighashType::All.to_u32() as u8);

            BitcoinSignature::from_inner(der_sig)
        })
        .collect::<Vec<_>>()
}

#[test]
fn instantiate() {
    let (mut suite, accounts, codes, ..) = setup_test_naive(Default::default());

    let mut owner = accounts.owner;
    let owner_address = owner.address.inner().clone();
    let bitcoin_hash = codes.bitcoin.to_bytes().hash256();

    let multisig_settings = MultisigSettings::new(
        2,
        NonEmpty::new(btree_set!(
            HexByteArray::from_str(MOCK_BRIDGE_GUARDIANS_KEYS[0].1).unwrap(),
            HexByteArray::from_str(MOCK_BRIDGE_GUARDIANS_KEYS[1].1).unwrap(),
            HexByteArray::from_str(MOCK_BRIDGE_GUARDIANS_KEYS[2].1).unwrap(),
        ))
        .unwrap(),
    )
    .unwrap();

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
            multisig: multisig_settings.clone(),
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
            multisig: multisig_settings.clone(),
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
            network: Network::Regtest,
            vault: MOCK_BITCOIN_REGTEST_VAULT.to_string(),
            guardians: NonEmpty::new_unchecked(btree_set!(
                accounts.user1.address.inner().clone(),
                accounts.user2.address.inner().clone(),
                accounts.user3.address.inner().clone(),
            )),
            multisig: multisig_settings.clone(),
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

    // Interact directly to the bridge (only gateway can).
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

    let val_sk1 = SecretKey::from_str(MOCK_BRIDGE_GUARDIANS_KEYS[0].0).unwrap();
    let val_pk1 = HexByteArray::<33>::from_str(MOCK_BRIDGE_GUARDIANS_KEYS[0].1).unwrap();

    let val_sk2 = SecretKey::from_str(MOCK_BRIDGE_GUARDIANS_KEYS[1].0).unwrap();
    let val_pk2 = HexByteArray::<33>::from_str(MOCK_BRIDGE_GUARDIANS_KEYS[1].1).unwrap();

    let config = suite
        .query_wasm_smart(contracts.bitcoin, QueryConfigRequest {})
        .should_succeed();

    let redeem_script = config.multisig.script();

    // Make 2 deposits and create a withdrawal.
    let deposit_amount1 = Uint128::new(7_000);
    let deposit_amount2 = Uint128::new(8_000);
    {
        deposit(
            &mut suite,
            contracts.bitcoin,
            &mut accounts,
            deposit_amount1,
            Some(user1_address),
            0,
        );

        deposit(
            &mut suite,
            contracts.bitcoin,
            &mut accounts,
            deposit_amount2,
            Some(user1_address),
            1,
        );

        // Create 2 withdrawal.
        withdraw(
            &mut suite,
            &mut accounts.user1,
            contracts.gateway,
            Uint128::new(10_000),
            "bcrt1q4e3mwznnr3chnytav5h4mhx52u447jv2kl55z9",
        );

        advance_ten_minutes(&mut suite);
    }

    // Retrieve the transaction.
    let tx = suite
        .query_wasm_smart(contracts.bitcoin, QueryOutboundTransactionRequest { id: 0 })
        .should_succeed();

    let btc_transaction = tx.to_btc_transaction(config.network).unwrap();

    let signatures1 = sing_inputs(&btc_transaction, &val_sk1, redeem_script, vec![
        deposit_amount1.into_inner() as u64,
        deposit_amount2.into_inner() as u64,
    ]);

    let signatures2 = sing_inputs(&btc_transaction, &val_sk2, redeem_script, vec![
        deposit_amount1.into_inner() as u64,
        deposit_amount2.into_inner() as u64,
    ]);

    // Ensure it fails with a invalid pubkey.
    {
        let msg = ExecuteMsg::AuthorizeOutbound {
            id: 0,
            signatures: vec![],
            pub_key: HexByteArray::<33>::from_slice(&[0; 33]).unwrap(),
        };

        suite
            .execute(&mut accounts.user1, contracts.bitcoin, &msg, Coins::new())
            .should_fail_with_error("is not a valid multisig public key");
    }

    // Ensure if fails with a wrong number of signatures.
    {
        let msg = ExecuteMsg::AuthorizeOutbound {
            id: 0,
            signatures: vec![],
            pub_key: val_pk1,
        };

        suite
            .execute(&mut accounts.val1, contracts.bitcoin, &msg, Coins::new())
            .should_fail_with_error("transaction `0` has 2 inputs, but 0 signatures were provided");
    }

    // Ensure it fails with a wrong combination pk and signature.
    {
        let msg = ExecuteMsg::AuthorizeOutbound {
            id: 0,
            signatures: signatures1.clone(),
            pub_key: val_pk2, // Using val_pk2 instead of val_pk1
        };

        suite
            .execute(&mut accounts.val1, contracts.bitcoin, &msg, Coins::new())
            .should_fail_with_error("signature failed verification");
    }

    // Ensure it fails with 1 signature correct and 1 not.
    {
        let msg = ExecuteMsg::AuthorizeOutbound {
            id: 0,
            signatures: vec![signatures1[0].clone(), signatures2[1].clone()],
            pub_key: val_pk1,
        };

        suite
            .execute(&mut accounts.val1, contracts.bitcoin, &msg, Coins::new())
            .should_fail_with_error("signature failed verification");
    }

    // Ensure it works with a correct signature and pk.
    {
        let msg = ExecuteMsg::AuthorizeOutbound {
            id: 0,
            signatures: signatures1.clone(),
            pub_key: val_pk1,
        };

        suite
            .execute(&mut accounts.val1, contracts.bitcoin, &msg, Coins::new())
            .should_succeed();
    }

    // Ensure it fails when trying to submit the same signature again.
    {
        let msg = ExecuteMsg::AuthorizeOutbound {
            id: 0,
            signatures: signatures1.clone(),
            pub_key: val_pk1,
        };

        suite
            .execute(&mut accounts.val1, contracts.bitcoin, &msg, Coins::new())
            .should_fail_with_error("you've already signed transaction `0`");
    }

    // Upload the second signatures and check for the event emitted.
    {
        let msg = ExecuteMsg::AuthorizeOutbound {
            id: 0,
            signatures: signatures2.clone(),
            pub_key: val_pk2,
        };

        let event = suite
            .execute(&mut accounts.val2, contracts.bitcoin, &msg, Coins::new())
            .should_succeed()
            .events
            .search_event::<CheckedContractEvent>()
            .with_predicate(|e| e.ty == "outbound_confirmed")
            .take()
            .one()
            .event
            .data
            .deserialize_json::<OutboundConfirmed>()
            .unwrap();

        assert_eq!(event, OutboundConfirmed {
            id: 0,
            transaction: tx,
            signatures: btree_map!(
                val_pk1 => signatures1,
                val_pk2 => signatures2,
            ),
        });
    }
}

#[test]
fn multisig_address() {
    let pk1 = HexByteArray::<33>::from_str(
        "029ba1aeddafb6ff65d403d50c0db0adbb8b5b3616c3bc75fb6fecd075327099f6",
    )
    .unwrap();
    let pk2 = HexByteArray::<33>::from_str(
        "03053780b7d8b3e7eb2771d7b9d43a946412e53fac90eadd46e214ccbea21eada6",
    )
    .unwrap();
    let pk3 = HexByteArray::<33>::from_str(
        "02f0bbe8928ab8d703e2e85093ee84ddfa9a0fdf48c443333098bd6188386bdb35",
    )
    .unwrap();

    let multisig =
        MultisigSettings::new(2, NonEmpty::new(btree_set!(pk1, pk2, pk3,)).unwrap()).unwrap();

    assert_eq!(
        multisig.address(Network::Regtest).to_string(),
        "bcrt1q4ga0r07vte2p638c8vh4fvpwjaln0qmxalffdkgeztl8l0act0xsvm7j9k"
    );
}
