use {
    grug_testing::TestBuilder,
    grug_types::{Coins, Message, NonZero, NumberConst, Uint256},
};

const DENOM: &str = "ugrug";

#[test]
fn transfers() -> anyhow::Result<()> {
    let (mut suite, accounts) = TestBuilder::new()
        .add_account("sender", Coins::one(DENOM, NonZero::new(100_u128)?))?
        .add_account("receiver", Coins::new())?
        .set_owner("sender")?
        .build()?;

    // Check that sender has been given 100 ugrug
    suite
        .query_balance(&accounts["sender"], DENOM)
        .should_succeed_and_equal(100_u128.into());
    suite
        .query_balance(&accounts["receiver"], DENOM)
        .should_succeed_and_equal(Uint256::ZERO);

    // Sender sends 70 ugrug to the receiver across multiple messages
    suite
        .send_messages(&accounts["sender"], vec![
            Message::Transfer {
                to: accounts["receiver"].address,
                coins: Coins::one(DENOM, NonZero::new(10_u128)?),
            },
            Message::Transfer {
                to: accounts["receiver"].address,
                coins: Coins::one(DENOM, NonZero::new(15_u128)?),
            },
            Message::Transfer {
                to: accounts["receiver"].address,
                coins: Coins::one(DENOM, NonZero::new(20_u128)?),
            },
            Message::Transfer {
                to: accounts["receiver"].address,
                coins: Coins::one(DENOM, NonZero::new(25_u128)?),
            },
        ])?
        .result
        .should_succeed();

    // Check balances again
    suite
        .query_balance(&accounts["sender"], DENOM)
        .should_succeed_and_equal(Uint256::from(30_u128));
    suite
        .query_balance(&accounts["receiver"], DENOM)
        .should_succeed_and_equal(Uint256::from(70_u128));

    Ok(())
}
