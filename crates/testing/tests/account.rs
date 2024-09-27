use {
    grug_math::{NumberConst, Uint256},
    grug_mock_account::Credential,
    grug_testing::TestBuilder,
    grug_types::{Coins, Duration, JsonDeExt, Message, ResultExt, Timestamp, Tx},
    grug_vm_rust::ContractBuilder,
};

#[test]
fn check_tx_and_finalize() -> anyhow::Result<()> {
    let (mut suite, mut accounts) = TestBuilder::new()
        .add_account("rhaki", [("uatom", Uint256::new_from_u128(100))])?
        .add_account("larry", Coins::new())?
        .add_account("owner", Coins::new())?
        .set_genesis_time(Timestamp::from_nanos(0))
        .set_block_time(Duration::from_seconds(1))
        .set_owner("owner")?
        .build()?;

    let transfer_msg = Message::transfer(
        accounts["larry"].address,
        Coins::one("uatom", Uint256::new_from_u128(10))?,
    )?;

    // Create a tx to set sequence to 1.
    suite.send_message(accounts.get_mut("rhaki").unwrap(), transfer_msg.clone())?;

    // Create a tx with sequence 0, 1, 2, 4.
    let txs: Vec<Tx> = [0, 1, 2, 4]
        .into_iter()
        .filter_map(|sequence| {
            (|| -> anyhow::Result<_> {
                // Sign the tx
                let tx = accounts["rhaki"].sign_transaction_with_sequence(
                    vec![transfer_msg.clone()],
                    &suite.chain_id,
                    sequence,
                    0,
                )?;

                // Check the tx and if the result is ok, return the tx.
                suite.check_tx(tx.clone())?.result.into_std_result()?;

                Ok(tx)
            })()
            .ok()
        })
        .collect();

    // The tx with sequence 0 should fails check_tx.
    assert_eq!(txs.len(), 3);
    assert_eq!(
        txs[0]
            .credential
            .clone()
            .deserialize_json::<Credential>()?
            .sequence,
        1
    );
    assert_eq!(
        txs[1]
            .credential
            .clone()
            .deserialize_json::<Credential>()?
            .sequence,
        2
    );
    assert_eq!(
        txs[2]
            .credential
            .clone()
            .deserialize_json::<Credential>()?
            .sequence,
        4
    );

    // Create a block with the txs.
    // The tx with sequence 1 should succeed.
    // The tx with sequence 2 should succeed.
    // The tx with sequence 4 should fail.
    let result = suite.make_block(txs)?;

    result.tx_outcomes[0].result.clone().should_succeed();
    result.tx_outcomes[1].result.clone().should_succeed();
    result.tx_outcomes[2].result.clone().should_fail();

    suite
        .query_balance(&accounts["rhaki"], "uatom")
        .should_succeed_and_equal(Uint256::new_from_u128(70));
    suite
        .query_balance(&accounts["larry"], "uatom")
        .should_succeed_and_equal(Uint256::new_from_u128(30));

    // Try create a block with a tx with sequence = 3
    let tx = accounts["rhaki"].sign_transaction_with_sequence(
        vec![transfer_msg],
        &suite.chain_id,
        3,
        0,
    )?;

    suite.make_block(vec![tx])?.tx_outcomes[0]
        .result
        .clone()
        .should_succeed();

    suite
        .query_balance(&accounts["rhaki"], "uatom")
        .should_succeed_and_equal(Uint256::new_from_u128(60));
    suite
        .query_balance(&accounts["larry"], "uatom")
        .should_succeed_and_equal(Uint256::new_from_u128(40));

    Ok(())
}

mod backrunner {
    use {
        grug_math::{Number, NumberConst, Uint128, Uint256},
        grug_types::{AuthCtx, AuthResponse, Coins, Denom, Message, Response, StdResult, Tx},
        std::str::FromStr,
    };

    // This contract is used for testing the backrunning feature, so we simply
    // skip all authentications in `authenticate`.
    pub fn authenticate(_ctx: AuthCtx, _tx: Tx) -> StdResult<AuthResponse> {
        // Do request backrunning.
        Ok(AuthResponse::new().request_backrun(true))
    }

    // Accounts can do any action while backrunning. In this test, the account
    // attempts to mint itself a token.
    pub fn backrun(ctx: AuthCtx, _tx: Tx) -> StdResult<Response> {
        let cfg = ctx.querier.query_config()?;

        Ok(Response::new().add_message(Message::execute(
            cfg.bank,
            &grug_mock_bank::ExecuteMsg::Mint {
                to: ctx.contract,
                denom: Denom::from_str("nft/badkids/1")?,
                amount: Uint256::ONE,
            },
            Coins::new(),
        )?))
    }

    // The account can also reject and revert state changes from the messages
    // simply by throwing en error while backrunning.
    pub fn bugged_backrun(_ctx: AuthCtx, _tx: Tx) -> StdResult<Response> {
        let _ = Uint128::ONE.checked_div(Uint128::ZERO)?;

        Ok(Response::new())
    }
}

#[test]
fn backrunning_works() -> anyhow::Result<()> {
    let account = ContractBuilder::new(Box::new(grug_mock_account::instantiate))
        .with_receive(Box::new(grug_mock_account::receive))
        .with_authenticate(Box::new(backrunner::authenticate))
        .with_backrun(Box::new(backrunner::backrun))
        .build();

    let (mut suite, mut accounts) = TestBuilder::new()
        .set_account_code(account, |public_key| grug_mock_account::InstantiateMsg {
            public_key,
        })?
        .add_account(
            "sender",
            Coins::one("ugrug", Uint256::new_from_u128(50_000))?,
        )?
        .add_account("receiver", Coins::new())?
        .set_owner("sender")?
        .build()?;

    let receiver = accounts["receiver"].address;

    // Attempt to send a transaction
    suite.transfer(
        accounts.get_mut("sender").unwrap(),
        receiver,
        Coins::one("ugrug", Uint256::new_from_u128(123))?,
    )?;

    // Receiver should have received ugrug, and sender should have minted bad kids.
    suite
        .query_balance(&accounts["receiver"], "ugrug")
        .should_succeed_and_equal(Uint256::new_from_u128(123));
    suite
        .query_balance(&accounts["sender"], "ugrug")
        .should_succeed_and_equal(Uint256::new_from_u128(50_000 - 123));
    suite
        .query_balance(&accounts["sender"], "nft/badkids/1")
        .should_succeed_and_equal(Uint256::ONE);

    Ok(())
}

#[test]
fn backrunning_with_error() -> anyhow::Result<()> {
    let bugged_account = ContractBuilder::new(Box::new(grug_mock_account::instantiate))
        .with_receive(Box::new(grug_mock_account::receive))
        .with_authenticate(Box::new(backrunner::authenticate))
        .with_backrun(Box::new(backrunner::bugged_backrun))
        .build();

    let (mut suite, mut accounts) = TestBuilder::new()
        .set_account_code(bugged_account, |public_key| {
            grug_mock_account::InstantiateMsg { public_key }
        })?
        .add_account(
            "sender",
            Coins::one("ugrug", Uint256::new_from_u128(50_000))?,
        )?
        .add_account("receiver", Coins::new())?
        .set_owner("sender")?
        .build()?;

    let receiver = accounts["receiver"].address;

    // Attempt to make a transfer; should fail.
    suite
        .send_message(
            accounts.get_mut("sender").unwrap(),
            Message::transfer(receiver, Coins::one("ugrug", Uint256::new_from_u128(123))?)?,
        )?
        .result
        .should_fail_with_error("division by zero: 1 / 0");

    // Transfer should have been reverted, and sender doesn't get bad kids.
    suite
        .query_balance(&accounts["receiver"], "ugrug")
        .should_succeed_and_equal(Uint256::ZERO);
    suite
        .query_balance(&accounts["sender"], "ugrug")
        .should_succeed_and_equal(Uint256::new_from_u128(50_000));
    suite
        .query_balance(&accounts["sender"], "nft/badkids/1")
        .should_succeed_and_equal(Uint256::ZERO);

    Ok(())
}
