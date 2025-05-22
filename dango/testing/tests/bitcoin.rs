use {
    corepc_client::bitcoin::Network,
    dango_testing::setup_test_naive,
    dango_types::bitcoin::{Config, ExecuteMsg, InstantiateMsg},
    grug::{
        Coins, ContractBuilder, Hash256, Message, NonEmpty, Order, ResultExt, Uint128, btree_set,
    },
    std::str::FromStr,
};

#[test]
fn instantiate() {
    let (mut suite, accounts, ..) = setup_test_naive(Default::default());

    let mut owner = accounts.owner;
    let owner_address = owner.address.inner().clone();

    let bitcoin_code = ContractBuilder::new(Box::new(dango_bitcoin::instantiate))
        .with_execute(Box::new(dango_bitcoin::execute))
        .with_query(Box::new(dango_bitcoin::query))
        .build();

    let bitcoin_hash = suite
        .upload(&mut owner, bitcoin_code)
        .should_succeed()
        .code_hash;

    // Try to instantiate the contract with wrong address.
    {
        let config = Config {
            network: Network::Bitcoin,
            vault: "48656c6c6f2044616e676f21".to_string(),
            guardians: NonEmpty::new_unchecked(btree_set!(
                accounts.user1.address.inner().clone(),
                accounts.user2.address.inner().clone(),
                accounts.user3.address.inner().clone(),
            )),
            threshold: 2,
            sats_per_vbyte: Uint128::new(10),
            outbound_fee: Uint128::new(10),
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
            outbound_fee: Uint128::new(10),
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
            outbound_fee: Uint128::new(10),
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
    let (mut suite, mut accounts, ..) = setup_test_naive(Default::default());

    let mut owner = accounts.owner;
    let owner_address = owner.address.inner().clone();

    let bitcoin_code = ContractBuilder::new(Box::new(dango_bitcoin::instantiate))
        .with_execute(Box::new(dango_bitcoin::execute))
        .with_query(Box::new(dango_bitcoin::query))
        .build();

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
        outbound_fee: Uint128::new(10),
        outbound_strategy: Order::Ascending,
    };

    let res = suite
        .upload_and_instantiate(
            &mut owner,
            bitcoin_code,
            &InstantiateMsg { config },
            "salt",
            Some("bitcoin_bridge"),
            Some(owner_address),
            Coins::new(),
        )
        .should_succeed();

    let contract = res.address;

    // Report a deposit.
    let msg = ExecuteMsg::ObserveInbound {
        transaction_hash: Hash256::from_str(
            "C42F8B7FEFBDDE209F16A3084D9A5B44913030322F3AF27459A980674A7B9356",
        )
        .unwrap(),
        vout: 1,
        amount: Uint128::new(100),
        recipient: None,
    };

    let msg = Message::execute(contract, &msg, Coins::new()).unwrap();

    // Broadcast the message with a non guardian signer.
    suite
        .send_message(&mut accounts.user4, msg.clone())
        .should_fail_with_error("you don't have the right, O you don't have the right");

    // Broadcast the message with first guardian signer.
    suite
        .send_message(&mut accounts.user1, msg.clone())
        .should_succeed();

    // Broadcast again the message with the same signer (should fail).
    suite
        .send_message(&mut accounts.user1, msg.clone())
        .should_fail_with_error("you've already voted for transaction");

    // Broadcast the message with second guardian signer.
    suite
        .send_message(&mut accounts.user2, msg.clone())
        .should_succeed();

    // Broadcast the message with third guardian signer
    // (should fail since already match the threshold).
    suite
        .send_message(&mut accounts.user3, msg.clone())
        .should_fail_with_error("already exists in UTXO set");
}
