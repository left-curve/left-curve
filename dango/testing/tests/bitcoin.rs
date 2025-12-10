use {
    corepc_client::bitcoin::{
        Amount, EcdsaSighashType, Network, Transaction as BtcTransaction, hashes::Hash,
        sighash::SighashCache,
    },
    dango_genesis::{BitcoinOption, GenesisOption},
    dango_testing::{
        MOCK_BITCOIN_REGTEST_VAULT, Preset, TestAccount, TestSuite, guardian1, guardian2,
        guardian3, setup_test_naive, setup_test_naive_with_custom_genesis,
    },
    dango_types::{
        bitcoin::{
            BitcoinSignature, Config, ExecuteMsg, InboundConfirmed, InboundCredential, InboundMsg,
            InstantiateMsg, MultisigSettings, MultisigWallet, OutboundConfirmed, OutboundRequested,
            QueryAccountsIndexRequest, QueryConfigRequest, QueryOutboundQueueRequest,
            QueryOutboundTransactionRequest, QueryUtxosRequest, Recipient, Utxo, Vout,
        },
        constants::btc,
        gateway::{
            self,
            bridge::{BridgeMsg, TransferRemoteRequest},
        },
    },
    grug::{
        Addr, Addressable, CheckedContractEvent, Coins, CommitmentStatus, Duration, EventStatus,
        Hash256, HashExt, HexBinary, HexByteArray, Inner, Json, JsonDeExt, JsonSerExt,
        MakeBlockOutcome, Message, NonEmpty, Order, PrimaryKey, QuerierExt, ResultExt, SearchEvent,
        Tx, TxOutcome, Uint128, btree_map, btree_set, coins,
    },
    grug_app::NaiveProposalPreparer,
    identity::Identity256,
    k256::ecdsa::{Signature, SigningKey, signature::DigestSigner},
    std::{str::FromStr, vec},
};

// Create and confirm a deposit to bitcoin bridge contract.
fn deposit(
    suite: &mut TestSuite<NaiveProposalPreparer>,
    bitcoin_contract: Addr,
    msg: InboundMsg,
    sk: &SigningKey,
) -> TxOutcome {
    let identity = Identity256::from(*msg.hash().unwrap().inner());
    let signature: Signature = sk.sign_digest(identity);

    let msg = Message::execute(
        bitcoin_contract,
        &ExecuteMsg::ObserveInbound(msg),
        Coins::new(),
    )
    .unwrap();

    let credential = InboundCredential {
        signature: HexBinary::from_inner(signature.to_der().as_bytes().to_vec()),
    };

    let tx = Tx {
        sender: bitcoin_contract,
        gas_limit: 5_000_000,
        msgs: NonEmpty::new_unchecked(vec![msg]),
        data: Json::null(),
        credential: credential.to_json_value().unwrap(),
    };

    suite.send_transaction(tx)
}

// Create a deposit and sign it with 2 guardians.
fn deposit_and_confirm(
    suite: &mut TestSuite<NaiveProposalPreparer>,
    bitcoin_contract: Addr,
    tx_hash: Hash256,
    vout: Vout,
    amount: Uint128,
    recipient: Recipient,
) {
    let val_sk1 = SigningKey::from_bytes(&guardian1::PRIVATE_KEY.into()).unwrap();
    let val_pk1 = HexByteArray::<33>::from_inner(guardian1::PUBLIC_KEY);

    let val_sk2 = SigningKey::from_bytes(&guardian2::PRIVATE_KEY.into()).unwrap();
    let val_pk2 = HexByteArray::<33>::from_inner(guardian2::PUBLIC_KEY);

    let msg = InboundMsg {
        transaction_hash: tx_hash,
        vout,
        amount,
        recipient: recipient.clone(),
        pub_key: val_pk1,
    };

    deposit(suite, bitcoin_contract, msg, &val_sk1).should_succeed();

    let msg = InboundMsg {
        transaction_hash: tx_hash,
        vout,
        amount,
        recipient,
        pub_key: val_pk2,
    };

    deposit(suite, bitcoin_contract, msg, &val_sk2).should_succeed();
}

// Create a withdrawal request from a user.
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
fn advance_ten_minutes(suite: &mut TestSuite<NaiveProposalPreparer>) -> MakeBlockOutcome {
    suite.block_time = Duration::from_minutes(10);
    let outcome = suite.make_empty_block();
    suite.block_time = Duration::ZERO;

    outcome
}

// Sign the inputs of a Bitcoin transaction with the given secret key and redeem script.
pub fn sign_inputs(
    tx: &BtcTransaction,
    sk: &SigningKey,
    multisig_settings: &MultisigSettings,
    input_data: Vec<(u64, Recipient)>,
) -> Vec<HexBinary> {
    let mut cache = SighashCache::new(tx);
    input_data
        .into_iter()
        .enumerate()
        .map(|(i, (amount, recipient))| {
            // Create the correct multisig.
            let multisig = MultisigWallet::new(multisig_settings, &recipient);

            let sighash = cache
                .p2wsh_signature_hash(
                    i,
                    multisig.script(),
                    Amount::from_sat(amount),
                    EcdsaSighashType::All, // To sign all inputs and outputs
                )
                .unwrap();

            let identity = Identity256::from(sighash.to_byte_array());
            let signature: Signature = sk.sign_digest(identity);
            let mut der_sig = signature.to_der().as_bytes().to_vec();
            der_sig.push(EcdsaSighashType::All.to_u32() as u8);

            BitcoinSignature::from_inner(der_sig)
        })
        .collect::<Vec<_>>()
}

// Create a deposit address for the user.
pub fn create_deposit_address(
    suite: &mut TestSuite<NaiveProposalPreparer>,
    bitcoin_contract: Addr,
    user: &mut TestAccount,
) -> (u64, TxOutcome) {
    // Query the next address index.
    let index = suite
        .query_wasm_smart(bitcoin_contract, QueryAccountsIndexRequest {})
        .should_succeed();

    let outcome = suite.execute(
        user,
        bitcoin_contract,
        &ExecuteMsg::CreateDepositAddress {},
        Coins::new(),
    );

    (index, outcome)
}

#[test]
fn instantiate() {
    let (mut suite, accounts, codes, ..) = setup_test_naive(Default::default());

    let mut owner = accounts.owner;
    let owner_address = *owner.address.inner();
    let bitcoin_hash = codes.bitcoin.to_bytes().hash256();

    let multisig_settings = MultisigSettings::new(
        2,
        NonEmpty::new(btree_set!(
            HexByteArray::from_inner(guardian1::PUBLIC_KEY),
            HexByteArray::from_inner(guardian2::PUBLIC_KEY),
            HexByteArray::from_inner(guardian3::PUBLIC_KEY),
        ))
        .unwrap(),
    )
    .unwrap();

    // Try to instantiate the contract with wrong address.
    {
        let config = Config {
            network: Network::Bitcoin,
            vault: "Hello Dango!".to_string(),
            multisig: multisig_settings.clone(),
            sats_per_vbyte: Uint128::new(10),
            fee_rate_updater: *owner.address.inner(),
            minimum_deposit: Uint128::new(1000),
            max_output_per_tx: 30,
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
            multisig: multisig_settings.clone(),
            sats_per_vbyte: Uint128::new(10),
            fee_rate_updater: *owner.address.inner(),
            minimum_deposit: Uint128::new(1000),
            max_output_per_tx: 30,
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
            multisig: multisig_settings.clone(),
            sats_per_vbyte: Uint128::new(10),
            fee_rate_updater: *owner.address.inner(),
            minimum_deposit: Uint128::new(1000),
            max_output_per_tx: 30,
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
fn authenticate() {
    let (mut suite, _, _, contracts, ..) = setup_test_naive(Default::default());

    let bitcoin_contract = contracts.bitcoin;

    let gas_limit = 5_000_000;

    let msg = ExecuteMsg::ObserveInbound(InboundMsg {
        transaction_hash: Hash256::from_inner([0; 32]),
        vout: 0,
        amount: Uint128::new(10_000),
        recipient: Recipient::Vault,
        pub_key: HexByteArray::from_slice(&[0; 33]).unwrap(),
    });

    // Ensure the tx fails if there isn't exactly 1 message.
    {
        let msg = Message::execute(bitcoin_contract, &msg, Coins::new()).unwrap();

        let tx = Tx {
            sender: bitcoin_contract,
            gas_limit,
            msgs: NonEmpty::new_unchecked(vec![msg.clone(), msg]),
            data: Json::null(),
            credential: Json::null(),
        };

        // Broadcast the tx
        suite
            .send_transaction(tx)
            .should_fail_with_error("transaction must contain exactly one message");
    }

    // Ensure the tx fails if the message call a contract different from the bridge.
    {
        let msg = Message::execute(contracts.gateway, &msg, Coins::new()).unwrap();

        let tx = Tx {
            sender: bitcoin_contract,
            gas_limit,
            msgs: NonEmpty::new_unchecked(vec![msg]),
            data: Json::null(),
            credential: Json::null(),
        };

        // Broadcast the tx
        suite
            .send_transaction(tx)
            .should_fail_with_error("contract must be the bitcoin bridge");
    }

    // Ensure that only `ObserveInbound` or `AuthorizeOutbound` can be called.
    // the execute message must be either `ObserveInbound` or `AuthorizeOutbound`
    {
        let msg = Message::execute(
            bitcoin_contract,
            &ExecuteMsg::UpdateConfig {
                fee_rate_updater: None,
                minimum_deposit: None,
                max_output_per_tx: None,
            },
            Coins::new(),
        )
        .unwrap();

        let tx = Tx {
            sender: bitcoin_contract,
            gas_limit,
            msgs: NonEmpty::new_unchecked(vec![msg]),
            data: Json::null(),
            credential: Json::null(),
        };

        // Broadcast the tx
        suite.send_transaction(tx).should_fail_with_error(
            "the execute message must be either `ObserveInbound` or `AuthorizeOutbound`",
        );
    }
}

#[test]
fn observe_inbound() {
    let (mut suite, mut accounts, _, contracts, ..) = setup_test_naive(Default::default());

    let val_sk1 = SigningKey::from_bytes(&guardian1::PRIVATE_KEY.into()).unwrap();
    let val_pk1 = HexByteArray::<33>::from_inner(guardian1::PUBLIC_KEY);

    let val_sk2 = SigningKey::from_bytes(&guardian2::PRIVATE_KEY.into()).unwrap();
    let val_pk2 = HexByteArray::<33>::from_inner(guardian2::PUBLIC_KEY);

    let val_sk3 = SigningKey::from_bytes(&guardian3::PRIVATE_KEY.into()).unwrap();
    let val_pk3 = HexByteArray::<33>::from_inner(guardian3::PUBLIC_KEY);

    // Signature checks.
    {
        // Ensure the message is rejected if the signature is wrong.
        let msg = InboundMsg {
            transaction_hash: Hash256::from_inner([0; 32]),
            vout: 1,
            amount: Uint128::new(100),
            recipient: Recipient::Vault,
            pub_key: val_pk2,
        };

        deposit(&mut suite, contracts.bitcoin, msg.clone(), &val_sk1)
            .should_fail_with_error("signature failed verification");

        // Ensure the message is rejected if the pubkey in not part of the set.
        let msg = InboundMsg {
            transaction_hash: Hash256::from_inner([0; 32]),
            vout: 1,
            amount: Uint128::new(100),
            recipient: Recipient::Vault,
            pub_key: HexByteArray::<33>::from_slice(&[0; 33]).unwrap(),
        };
        deposit(&mut suite, contracts.bitcoin, msg, &val_sk1)
            .should_fail_with_error("is not a valid multisig public key");
    }

    // Ensure the message is rejected if the amount is lower than the minimum deposit.
    {
        let msg = InboundMsg {
            transaction_hash: Hash256::from_inner([0; 32]),
            vout: 1,
            amount: Uint128::new(100),
            recipient: Recipient::Vault,
            pub_key: val_pk1,
        };

        deposit(&mut suite, contracts.bitcoin, msg, &val_sk1)
            .should_fail_with_error("minimum deposit not met");
    }

    // Deposit to a Recipient::Index address.
    let (address_index, outcome) =
        create_deposit_address(&mut suite, contracts.bitcoin, &mut accounts.user1);
    outcome.should_succeed();

    let bitcoin_tx_hash =
        Hash256::from_str("C42F8B7FEFBDDE209F16A3084D9A5B44913030322F3AF27459A980674A7B9356")
            .unwrap();
    let vout = 1;
    let amount = Uint128::new(2000);
    let recipient = Recipient::Index(address_index);

    let mut msg = InboundMsg {
        transaction_hash: bitcoin_tx_hash,
        vout,
        amount,
        recipient: recipient.clone(),
        pub_key: val_pk1,
    };

    // Broadcast the message with first guardian signer.
    deposit(&mut suite, contracts.bitcoin, msg.clone(), &val_sk1).should_succeed();

    // Broadcast again the message with the same signer (should fail since already voted).
    deposit(&mut suite, contracts.bitcoin, msg.clone(), &val_sk1)
        .should_fail_with_error("you've already voted for transaction");

    // Broadcast the message with second guardian signer.
    // The threshold is met so there should be the event.
    msg.pub_key = val_pk2;

    deposit(&mut suite, contracts.bitcoin, msg.clone(), &val_sk2)
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
            vout,
            amount,
            recipient,
        });

    // Ensure the user has received the deposit.
    let balance = suite
        .query_balance(accounts.user1.address.inner(), btc::DENOM.clone())
        .unwrap();
    assert_eq!(
        balance, amount,
        "recipient has wrong btc balance! expecting: {amount}, found: {balance}",
    );

    // Broadcast the message with third guardian signer
    // (should fail since already match the threshold).
    msg.pub_key = val_pk3;
    deposit(&mut suite, contracts.bitcoin, msg.clone(), &val_sk3)
        .should_fail_with_error("already exists in UTXO set");

    // Deposit to Recipient::Address address.
    {
        let tx_hash =
            Hash256::from_str("14A0BF02F69BD13C274ED22E20C1BF4CC5DABF99753DB32E5B8959BF4C5F1F5C")
                .unwrap();
        let vout = 1;
        let recipient = Recipient::Address(accounts.user2.address());

        let mut msg = InboundMsg {
            transaction_hash: tx_hash,
            vout,
            amount,
            recipient: recipient.clone(),
            pub_key: val_pk1,
        };

        // Broadcast with first guardian.
        deposit(&mut suite, contracts.bitcoin, msg.clone(), &val_sk1).should_succeed();

        // Broadcast with the second guardian.
        msg.pub_key = val_pk2;

        deposit(&mut suite, contracts.bitcoin, msg.clone(), &val_sk2)
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
                vout,
                amount,
                recipient,
            });

        // Ensure the user has received the deposit.
        let balance = suite
            .query_balance(&accounts.user2.address(), btc::DENOM.clone())
            .unwrap();
        assert_eq!(
            balance, amount,
            "recipient has wrong btc balance! expecting: {amount}, found: {balance}",
        );
    }

    // Deposit to Recipient::Vault
    {
        let tx_hash =
            Hash256::from_str("14A0BF02F69BD13C274ED22E20C1BF4CC5DABF99753DB32E5B8959BF4C5F1F5C")
                .unwrap();
        let vout = 2;
        let msg = InboundMsg {
            transaction_hash: tx_hash,
            vout,
            amount,
            recipient: Recipient::Vault,
            pub_key: val_pk1,
        };

        // Broadcast with first guardian.
        deposit(&mut suite, contracts.bitcoin, msg.clone(), &val_sk1).should_succeed();

        // Broadcast with the second guardian.
        let msg = InboundMsg {
            transaction_hash: tx_hash,
            vout,
            amount,
            recipient: Recipient::Vault,
            pub_key: val_pk2,
        };
        deposit(&mut suite, contracts.bitcoin, msg.clone(), &val_sk2)
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
                vout,
                amount,
                recipient: Recipient::Vault,
            });
    }
}

#[test]
fn same_hash_different_vout() {
    let (mut suite, mut accounts, _, contracts, ..) = setup_test_naive(Default::default());

    suite.block_time = Duration::from_minutes(10);

    // Make 2 deposits with the same transaction hash but different vout.
    let hash = Hash256::from_inner([0; 32]);
    let amount1 = Uint128::new(10_000);
    let amount2 = Uint128::new(20_000);

    let (address_index, outcome) =
        create_deposit_address(&mut suite, contracts.bitcoin, &mut accounts.user1);
    outcome.should_succeed();

    deposit_and_confirm(
        &mut suite,
        contracts.bitcoin,
        hash,
        0,
        amount1,
        Recipient::Index(address_index),
    );

    deposit_and_confirm(
        &mut suite,
        contracts.bitcoin,
        hash,
        1,
        amount2,
        Recipient::Index(address_index),
    );

    // Ensure there are the 2 deposits in the utxo.
    suite
        .query_wasm_smart(contracts.bitcoin, QueryUtxosRequest {
            start_after: None,
            limit: None,
            order: Order::Ascending,
        })
        .should_succeed_and_equal(vec![
            Utxo {
                transaction_hash: hash,
                vout: 0,
                amount: amount1,
            },
            Utxo {
                transaction_hash: hash,
                vout: 1,
                amount: amount2,
            },
        ]);

    // Create a withdrawal request for the first vout.
    let recipient = "bcrt1q8qzecux6rz9aatnpjulmfrraznyqjc3crq33m0";
    let withdraw_amount = Uint128::new(25_000);
    withdraw(
        &mut suite,
        &mut accounts.user1,
        contracts.gateway,
        withdraw_amount,
        recipient,
    );

    let withdraw_fee = suite
        .query_wasm_smart(contracts.gateway, gateway::QueryWithdrawalFeeRequest {
            denom: btc::DENOM.clone(),
            remote: gateway::Remote::Bitcoin,
        })
        .unwrap()
        .unwrap();

    let vault = suite
        .query_wasm_smart(contracts.bitcoin, QueryConfigRequest {})
        .unwrap()
        .vault;

    // Ensure the withdrawal is stored in the outbound queue.
    let tx = suite
        .query_wasm_smart(contracts.bitcoin, QueryOutboundTransactionRequest { id: 0 })
        .unwrap();

    assert_eq!(
        tx.inputs,
        btree_map!(
            (hash, 0) => (amount1, Recipient::Index(address_index)),
            (hash, 1) => (amount2, Recipient::Index(address_index)),
        )
    );

    // Ensure the inputs and outputs are correct.
    assert_eq!(tx.outputs.len(), 2);
    assert!(tx.outputs.contains_key(&vault));
    assert_eq!(
        tx.outputs.get(recipient).unwrap().clone(),
        withdraw_amount - withdraw_fee,
    );
}

#[test]
fn transfer_remote() {
    let (mut suite, mut accounts, _, contracts, ..) = setup_test_naive(Default::default());

    let btc_recipient = "bcrt1q8qzecux6rz9aatnpjulmfrraznyqjc3crq33m0".to_string();

    let (address_index, outcome) =
        create_deposit_address(&mut suite, contracts.bitcoin, &mut accounts.user1);
    outcome.should_succeed();

    // Deposit 100k sats to user1
    deposit_and_confirm(
        &mut suite,
        contracts.bitcoin,
        Hash256::from_inner([0; 32]),
        0,
        Uint128::new(100_000),
        Recipient::Index(address_index),
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

    let (address_index, outcome) =
        create_deposit_address(&mut suite, contracts.bitcoin, &mut accounts.user1);
    outcome.should_succeed();

    let withdraw_fee = suite
        .query_wasm_smart(contracts.gateway, gateway::QueryWithdrawalFeeRequest {
            denom: btc::DENOM.clone(),
            remote: gateway::Remote::Bitcoin,
        })
        .unwrap()
        .unwrap();

    // Deposit 100k sats do user1
    deposit_and_confirm(
        &mut suite,
        contracts.bitcoin,
        Hash256::from_inner([0; 32]),
        0,
        Uint128::new(100_000),
        Recipient::Index(address_index),
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
    let outcome = advance_ten_minutes(&mut suite);
    println!("{:#?}", outcome.block_outcome.cron_outcomes);

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
        btree_map!( (Hash256::from_inner([0u8; 32]), 0) => (Uint128::new(100_000), Recipient::Index(address_index)) )
    );

    assert_eq!(
        tx.outputs,
        btree_map!(
            recipient1.clone() => net_withdraw1,
            recipient2.clone() => net_withdraw2,
            vault => Uint128::new(100_000) - net_withdraw1 - net_withdraw2 - tx.fee
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

    // Make another withdrawal. Now, there are no more UTXO available since
    // the only one is already used. The cron job should fail.
    withdraw(
        &mut suite,
        &mut accounts.user1,
        contracts.gateway,
        Uint128::new(10_000),
        &recipient2,
    );

    // Wait for the cron job to execute.
    let outcome = advance_ten_minutes(&mut suite);

    let events = outcome
        .block_outcome
        .cron_outcomes
        .iter()
        .filter_map(|co| {
            if let Err((event_status, error)) = co.cron_event.as_result() {
                match event_status {
                    grug::EventStatus::NestedFailed(event) => Some((event, error)),
                    _ => None,
                }
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].0.contract, contracts.bitcoin);
    assert!(
        events[0]
            .1
            .to_string()
            .contains("not enough UTXOs to cover the withdraw amount + fee")
    );
}

#[test]
fn authorize_outbound() {
    let (mut suite, mut accounts, _, contracts, ..) = setup_test_naive(Default::default());

    suite.block_time = Duration::ZERO;

    let config = suite
        .query_wasm_smart(contracts.bitcoin, QueryConfigRequest {})
        .should_succeed();

    let sk1 = SigningKey::from_bytes(&guardian1::PRIVATE_KEY.into()).unwrap();
    let pk1 = HexByteArray::<33>::from_inner(guardian1::PUBLIC_KEY);

    let sk2 = SigningKey::from_bytes(&guardian2::PRIVATE_KEY.into()).unwrap();
    let pk2 = HexByteArray::<33>::from_inner(guardian2::PUBLIC_KEY);

    let sk3 = SigningKey::from_bytes(&guardian3::PRIVATE_KEY.into()).unwrap();
    let pk3 = HexByteArray::<33>::from_inner(guardian3::PUBLIC_KEY);

    let (address_index, outcome) =
        create_deposit_address(&mut suite, contracts.bitcoin, &mut accounts.user1);
    outcome.should_succeed();

    // Make 2 deposits and create a withdrawal.
    let deposit_amount1 = Uint128::new(7_000);
    let deposit_amount2 = Uint128::new(8_000);
    {
        deposit_and_confirm(
            &mut suite,
            contracts.bitcoin,
            Hash256::from_inner([0; 32]),
            0,
            deposit_amount1,
            Recipient::Index(address_index),
        );

        deposit_and_confirm(
            &mut suite,
            contracts.bitcoin,
            Hash256::from_inner([1; 32]),
            0,
            deposit_amount2,
            Recipient::Index(address_index),
        );

        // Create 2 withdrawals.
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
    let outbound_tx = suite
        .query_wasm_smart(contracts.bitcoin, QueryOutboundTransactionRequest { id: 0 })
        .should_succeed();

    let btc_transaction = outbound_tx.to_btc_transaction(config.network).unwrap();

    let signatures1 = sign_inputs(&btc_transaction, &sk1, &config.multisig, vec![
        (
            deposit_amount1.into_inner() as u64,
            Recipient::Index(address_index),
        ),
        (
            deposit_amount2.into_inner() as u64,
            Recipient::Index(address_index),
        ),
    ]);

    let signatures2 = sign_inputs(&btc_transaction, &sk2, &config.multisig, vec![
        (
            deposit_amount1.into_inner() as u64,
            Recipient::Index(address_index),
        ),
        (
            deposit_amount2.into_inner() as u64,
            Recipient::Index(address_index),
        ),
    ]);

    let signatures3 = sign_inputs(&btc_transaction, &sk3, &config.multisig, vec![
        (
            deposit_amount1.into_inner() as u64,
            Recipient::Index(address_index),
        ),
        (
            deposit_amount2.into_inner() as u64,
            Recipient::Index(address_index),
        ),
    ]);

    // Ensure no one can call `AuthorizeOutbound` except bitcoin bridge.
    {
        let msg = ExecuteMsg::AuthorizeOutbound {
            id: 0,
            signatures: signatures1.clone(),
            pub_key: pk1,
        };

        suite
            .execute(&mut accounts.user1, contracts.bitcoin, &msg, Coins::new())
            .should_fail_with_error("you don't have the right, O you don't have the right");
    }

    // Ensure it fails with a invalid pubkey.
    {
        let msg = ExecuteMsg::AuthorizeOutbound {
            id: 0,
            signatures: vec![],
            pub_key: HexByteArray::<33>::from_slice(&[0; 33]).unwrap(),
        };

        let msg = Message::execute(contracts.bitcoin, &msg, Coins::new()).unwrap();

        let tx = Tx {
            sender: contracts.bitcoin,
            gas_limit: 5_000_000,
            msgs: NonEmpty::new_unchecked(vec![msg]),
            data: Json::null(),
            credential: Json::null(),
        };

        suite
            .send_transaction(tx)
            .should_fail_with_error("is not a valid multisig public key");
    }

    // Ensure if fails with a wrong number of signatures.
    {
        let msg = ExecuteMsg::AuthorizeOutbound {
            id: 0,
            signatures: vec![],
            pub_key: pk1,
        };

        let msg = Message::execute(contracts.bitcoin, &msg, Coins::new()).unwrap();

        let tx = Tx {
            sender: contracts.bitcoin,
            gas_limit: 5_000_000,
            msgs: NonEmpty::new_unchecked(vec![msg]),
            data: Json::null(),
            credential: Json::null(),
        };

        suite
            .send_transaction(tx)
            .should_fail_with_error("transaction `0` has 2 inputs, but 0 signatures were provided");
    }

    // Ensure it fails with a wrong combination pk and signature.
    {
        let msg = ExecuteMsg::AuthorizeOutbound {
            id: 0,
            signatures: signatures1.clone(),
            pub_key: pk2, // Using val_pk2 instead of val_pk1
        };

        let msg = Message::execute(contracts.bitcoin, &msg, Coins::new()).unwrap();

        let tx = Tx {
            sender: contracts.bitcoin,
            gas_limit: 5_000_000,
            msgs: NonEmpty::new_unchecked(vec![msg]),
            data: Json::null(),
            credential: Json::null(),
        };

        suite
            .send_transaction(tx)
            .should_fail_with_error("signature failed verification");
    }

    // Ensure it fails with 1 signature correct and 1 not.
    {
        let msg = ExecuteMsg::AuthorizeOutbound {
            id: 0,
            signatures: vec![signatures1[0].clone(), signatures2[1].clone()],
            pub_key: pk1,
        };

        let msg = Message::execute(contracts.bitcoin, &msg, Coins::new()).unwrap();

        let tx = Tx {
            sender: contracts.bitcoin,
            gas_limit: 5_000_000,
            msgs: NonEmpty::new_unchecked(vec![msg]),
            data: Json::null(),
            credential: Json::null(),
        };

        suite
            .send_transaction(tx)
            .should_fail_with_error("signature failed verification");
    }

    // Ensure it works with a correct signature and pk.
    {
        let msg = ExecuteMsg::AuthorizeOutbound {
            id: 0,
            signatures: signatures1.clone(),
            pub_key: pk1,
        };

        let msg = Message::execute(contracts.bitcoin, &msg, Coins::new()).unwrap();

        let tx = Tx {
            sender: contracts.bitcoin,
            gas_limit: 5_000_000,
            msgs: NonEmpty::new_unchecked(vec![msg]),
            data: Json::null(),
            credential: Json::null(),
        };

        suite.send_transaction(tx).should_succeed();
    }

    // Ensure it fails when trying to submit the same signature again.
    {
        let msg = ExecuteMsg::AuthorizeOutbound {
            id: 0,
            signatures: signatures1.clone(),
            pub_key: pk1,
        };

        let msg = Message::execute(contracts.bitcoin, &msg, Coins::new()).unwrap();

        let tx = Tx {
            sender: contracts.bitcoin,
            gas_limit: 5_000_000,
            msgs: NonEmpty::new_unchecked(vec![msg]),
            data: Json::null(),
            credential: Json::null(),
        };

        suite
            .send_transaction(tx)
            .should_fail_with_error("you've already signed transaction `0`");
    }

    // Upload the second signatures and check for the event emitted.
    {
        let msg = ExecuteMsg::AuthorizeOutbound {
            id: 0,
            signatures: signatures2.clone(),
            pub_key: pk2,
        };

        let msg = Message::execute(contracts.bitcoin, &msg, Coins::new()).unwrap();

        let tx = Tx {
            sender: contracts.bitcoin,
            gas_limit: 5_000_000,
            msgs: NonEmpty::new_unchecked(vec![msg]),
            data: Json::null(),
            credential: Json::null(),
        };

        let event = suite
            .send_transaction(tx)
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
            transaction: outbound_tx,
            signatures: btree_map!(
                pk1 => signatures1,
                pk2 => signatures2,
            ),
        });
    }

    // Ensure it fails trying to upload another signature when the threshold is already met.
    {
        let msg = ExecuteMsg::AuthorizeOutbound {
            id: 0,
            signatures: signatures3,
            pub_key: pk3,
        };

        let msg = Message::execute(contracts.bitcoin, &msg, Coins::new()).unwrap();

        let tx = Tx {
            sender: contracts.bitcoin,
            gas_limit: 5_000_000,
            msgs: NonEmpty::new_unchecked(vec![msg]),
            data: Json::null(),
            credential: Json::null(),
        };

        suite
            .send_transaction(tx)
            .should_fail_with_error("transaction `0` already has enough signatures");
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

    let multisig_settings =
        MultisigSettings::new(2, NonEmpty::new(btree_set!(pk1, pk2, pk3,)).unwrap()).unwrap();

    let multisig = MultisigWallet::new(&multisig_settings, &Recipient::Vault);

    assert_eq!(
        multisig.address(Network::Regtest).to_string(),
        "bcrt1q4ga0r07vte2p638c8vh4fvpwjaln0qmxalffdkgeztl8l0act0xsvm7j9k"
    );
}

#[test]
fn fee() {
    let (mut suite, mut accounts, _, contracts, ..) = setup_test_naive(Default::default());

    suite.block_time = Duration::from_minutes(10);

    let sk1 = SigningKey::from_bytes(&guardian1::PRIVATE_KEY.into()).unwrap();
    let sk2 = SigningKey::from_bytes(&guardian2::PRIVATE_KEY.into()).unwrap();

    let btc_recipient = "bcrt1q8qzecux6rz9aatnpjulmfrraznyqjc3crq33m0";

    let (address_index, outcome) =
        create_deposit_address(&mut suite, contracts.bitcoin, &mut accounts.user1);
    outcome.should_succeed();

    let bitcoin_config = suite
        .query_wasm_smart(contracts.bitcoin, QueryConfigRequest {})
        .unwrap();

    let multisig_wallet = MultisigWallet::new(&bitcoin_config.multisig, &Recipient::Vault);

    // Create 2 deposits to user1.
    let amount1 = Uint128::new(20_000);
    deposit_and_confirm(
        &mut suite,
        contracts.bitcoin,
        Hash256::from_inner([0; 32]),
        0,
        amount1,
        Recipient::Index(address_index),
    );

    let amount2 = Uint128::new(30_000);
    deposit_and_confirm(
        &mut suite,
        contracts.bitcoin,
        Hash256::from_inner([1; 32]),
        0,
        amount2,
        Recipient::Index(address_index),
    );

    // Create a withdrawal request.
    withdraw(
        &mut suite,
        &mut accounts.user1,
        contracts.gateway,
        Uint128::new(40_000),
        btc_recipient,
    );

    // Build the transaction and add the signatures.
    let tx = suite
        .query_wasm_smart(contracts.bitcoin, QueryOutboundTransactionRequest { id: 0 })
        .unwrap();

    let mut btc_tx = tx.to_btc_transaction(bitcoin_config.network).unwrap();

    let signature1 = sign_inputs(&btc_tx, &sk1, &bitcoin_config.multisig, vec![
        (amount1.into_inner() as u64, Recipient::Index(address_index)),
        (amount2.into_inner() as u64, Recipient::Index(address_index)),
    ]);

    let signature2 = sign_inputs(&btc_tx, &sk2, &bitcoin_config.multisig, vec![
        (amount1.into_inner() as u64, Recipient::Index(address_index)),
        (amount2.into_inner() as u64, Recipient::Index(address_index)),
    ]);

    for i in 0..btc_tx.input.len() {
        let input = btc_tx.input.get_mut(i).unwrap();

        input.witness.push(vec![]);
        input.witness.push(&signature1[i]);
        input.witness.push(&signature2[i]);
        input.witness.push(multisig_wallet.script());
    }

    // Real fee for the tx.
    let fee = Uint128::new(btc_tx.vsize() as u128) * bitcoin_config.sats_per_vbyte;

    // Fee estimated by the contract.
    let fee_estimation = tx.fee;

    assert!(fee_estimation >= fee);

    // Ensure the fee estimation is not too high.
    let percentage = fee_estimation * Uint128::new(100) / fee;
    println!("Percentage: {percentage}");
    assert!(percentage < Uint128::new(105));
}

#[test]
fn multiple_outbound_tx() {
    let genesis_option = GenesisOption {
        bitcoin: BitcoinOption {
            max_output_per_tx: 1,
            ..Preset::preset_test()
        },
        ..Preset::preset_test()
    };

    let (mut suite, mut accounts, _, contracts, ..) =
        setup_test_naive_with_custom_genesis(Preset::preset_test(), genesis_option);

    suite.block_time = Duration::ZERO;

    let (address_index_user1, outcome) =
        create_deposit_address(&mut suite, contracts.bitcoin, &mut accounts.user1);
    outcome.should_succeed();

    let (address_index_user2, outcome) =
        create_deposit_address(&mut suite, contracts.bitcoin, &mut accounts.user2);
    outcome.should_succeed();

    let hash1 = Hash256::from_inner([0; 32]);
    let hash2 = Hash256::from_inner([1; 32]);

    // Deposit 100k sats to user1
    deposit_and_confirm(
        &mut suite,
        contracts.bitcoin,
        hash1,
        0,
        Uint128::new(100_000),
        Recipient::Index(address_index_user1),
    );

    // Deposit 100k sats to user2
    deposit_and_confirm(
        &mut suite,
        contracts.bitcoin,
        hash2,
        0,
        Uint128::new(100_000),
        Recipient::Index(address_index_user2),
    );

    // Create a withdrawal request for user1 and user2.
    let recipient1 = "bcrt1q4e3mwznnr3chnytav5h4mhx52u447jv2kl55z9";
    let recipient2 = "bcrt1q8qzecux6rz9aatnpjulmfrraznyqjc3crq33m0";

    withdraw(
        &mut suite,
        &mut accounts.user1,
        contracts.gateway,
        Uint128::new(10_000),
        recipient1,
    );

    withdraw(
        &mut suite,
        &mut accounts.user1,
        contracts.gateway,
        Uint128::new(10_000),
        recipient2,
    );

    // Let cronjob execute the withdrawals and ensure there are 2 events.
    let outcome = advance_ten_minutes(&mut suite);

    let mut events = vec![];
    for cron in outcome.block_outcome.cron_outcomes {
        if let CommitmentStatus::Committed(EventStatus::Ok(cron_event)) = &cron.cron_event
            && cron_event.contract == contracts.bitcoin
            && let EventStatus::Ok(guest) = &cron_event.guest_event
        {
            events.extend(
                guest
                    .contract_events
                    .iter()
                    .map(|e| {
                        e.data
                            .clone()
                            .deserialize_json::<OutboundRequested>()
                            .unwrap()
                    })
                    .collect::<Vec<_>>(),
            );
        }
    }

    assert_eq!(events.len(), 2);

    assert_eq!(events[0].id, 0);

    assert_eq!(events[1].id, 1);

    // Ensure the outbound queue is empty.
    suite
        .query_wasm_smart(contracts.bitcoin, QueryOutboundQueueRequest {
            start_after: None,
            limit: None,
        })
        .should_succeed_and_equal(btree_map!());

    let tx1 = suite
        .query_wasm_smart(contracts.bitcoin, QueryOutboundTransactionRequest { id: 0 })
        .should_succeed();

    assert_eq!(tx1.inputs, btree_map! {
        (hash1, 0) => (Uint128::new(100_000), Recipient::Index(address_index_user1)),
    });

    assert!(tx1.outputs.contains_key(recipient1));

    let tx2 = suite
        .query_wasm_smart(contracts.bitcoin, QueryOutboundTransactionRequest { id: 1 })
        .should_succeed();

    assert_eq!(tx2.inputs, btree_map! {
        (hash2, 0) => (Uint128::new(100_000), Recipient::Index(address_index_user2)),
    });

    assert!(tx2.outputs.contains_key(recipient2));
}

#[test]
fn update_fee_rate() {
    let (mut suite, mut accounts, _, contracts, ..) = setup_test_naive(Default::default());

    let current_fee_rate = suite
        .query_wasm_smart(contracts.bitcoin, QueryConfigRequest {})
        .unwrap()
        .sats_per_vbyte;

    let new_fee_rate = current_fee_rate * Uint128::new(2);

    let msg = ExecuteMsg::UpdateFeeRate(new_fee_rate);

    // Try to update the price with a non authorized address.
    suite
        .execute(&mut accounts.user1, contracts.bitcoin, &msg, Coins::new())
        .should_fail_with_error("you don't have the right, O you don't have the right");

    // Update with an authorized address.
    suite
        .execute(&mut accounts.owner, contracts.bitcoin, &msg, Coins::new())
        .should_succeed();

    // Ensure the price is updated.
    assert_eq!(
        suite
            .query_wasm_smart(contracts.bitcoin, QueryConfigRequest {})
            .unwrap()
            .sats_per_vbyte,
        new_fee_rate
    );
}

#[test]
fn update_config() {
    let (mut suite, mut accounts, _, contracts, ..) = setup_test_naive(Default::default());

    let config = suite
        .query_wasm_smart(contracts.bitcoin, QueryConfigRequest {})
        .unwrap();

    let new_fee_rate_updater = *accounts.user1.address.inner();
    let new_minimum_deposit = Uint128::new(200_000);
    let new_max_output_per_tx = 1000;

    assert_ne!(config.fee_rate_updater, new_fee_rate_updater);
    assert_ne!(config.minimum_deposit, new_minimum_deposit);
    assert_ne!(config.max_output_per_tx, new_max_output_per_tx);

    // Try to update the config with a non authorized address.
    let msg = ExecuteMsg::UpdateConfig {
        fee_rate_updater: Some(new_fee_rate_updater),
        minimum_deposit: Some(new_minimum_deposit),
        max_output_per_tx: Some(new_max_output_per_tx),
    };

    // Ensure only owner can update the config.
    suite
        .execute(&mut accounts.user1, contracts.bitcoin, &msg, Coins::new())
        .should_fail_with_error("you don't have the right, O you don't have the right");

    // Update with an authorized address.
    suite
        .execute(&mut accounts.owner, contracts.bitcoin, &msg, Coins::new())
        .should_succeed();

    // Ensure the config is updated.
    let config = suite
        .query_wasm_smart(contracts.bitcoin, QueryConfigRequest {})
        .should_succeed();

    assert_eq!(config.fee_rate_updater, new_fee_rate_updater);
    assert_eq!(config.minimum_deposit, new_minimum_deposit);
    assert_eq!(config.max_output_per_tx, new_max_output_per_tx);
}

#[test]
fn deposit_address_request() {
    let (mut suite, mut accounts, _, contracts, ..) = setup_test_naive(Default::default());

    let (id1, outcome) = create_deposit_address(&mut suite, contracts.bitcoin, &mut accounts.user1);
    outcome.should_succeed();

    let (_, outcome) = create_deposit_address(&mut suite, contracts.bitcoin, &mut accounts.user1);
    outcome.should_fail_with_error("you already have a deposit address");

    let (id2, outcome) = create_deposit_address(&mut suite, contracts.bitcoin, &mut accounts.user2);
    outcome.should_succeed();

    assert_eq!(id2, id1 + 1);
}
