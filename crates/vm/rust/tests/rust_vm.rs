use {
    grug_testing::TestBuilder,
    grug_types::{Coin, Coins, Message, NumberConst, Uint128},
};

const DENOM: &str = "ugrug";

#[test]
fn bank_transfers() -> anyhow::Result<()> {
    let (mut suite, accounts) = TestBuilder::new()
        .add_account("sender", Coins::new_one(DENOM, 100_u128))?
        .add_account("receiver", Coins::new_empty())?
        .build()?;

    // Check that sender has been given 100 ugrug
    suite
        .query_balance(&accounts["sender"], DENOM)
        .should_succeed_and_equal(Uint128::new(100))?;
    suite
        .query_balance(&accounts["receiver"], DENOM)
        .should_succeed_and_equal(Uint128::ZERO)?;

    // Sender sends 70 ugrug to the receiver across multiple messages
    suite
        .execute_messages(&accounts["sender"], 2_500_000, vec![
            Message::Transfer {
                to: accounts["receiver"].address.clone(),
                coins: vec![Coin::new(DENOM, 10_u128)].try_into().unwrap(),
            },
            Message::Transfer {
                to: accounts["receiver"].address.clone(),
                coins: vec![Coin::new(DENOM, 15_u128)].try_into().unwrap(),
            },
            Message::Transfer {
                to: accounts["receiver"].address.clone(),
                coins: vec![Coin::new(DENOM, 20_u128)].try_into().unwrap(),
            },
            Message::Transfer {
                to: accounts["receiver"].address.clone(),
                coins: vec![Coin::new(DENOM, 25_u128)].try_into().unwrap(),
            },
        ])?
        .should_succeed()?;

    // Check balances again
    suite
        .query_balance(&accounts["sender"], DENOM)
        .should_succeed_and_equal(Uint128::new(30))?;
    suite
        .query_balance(&accounts["receiver"], DENOM)
        .should_succeed_and_equal(Uint128::new(70))?;

    Ok(())
}
