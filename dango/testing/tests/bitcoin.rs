use {
    corepc_client::bitcoin::Network,
    dango_testing::setup_test_naive,
    dango_types::{
        bitcoin::{Config, ExecuteMsg, InboundConfirmed, InstantiateMsg},
        constants::btc,
    },
    grug::{
        CheckedContractEvent, Coins, Hash256, HashExt, JsonDeExt, Message, NonEmpty, Order,
        ResultExt, SearchEvent, Uint128, btree_set,
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
            .should_fail();
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
