use {
    grug_math::{NumberConst, Uint128},
    grug_testing::TestBuilder,
    grug_types::{Coins, Denom, Message, ResultExt},
    std::{str::FromStr, sync::LazyLock},
};

static DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("ugrug").unwrap());

#[test]
fn transfers() {
    let (mut suite, mut accounts) = TestBuilder::new()
        .add_account("sender", Coins::one(DENOM.clone(), 100).unwrap())
        .add_account("receiver", Coins::new())
        .set_owner("sender")
        .build();

    let to = accounts["receiver"].address;

    // Check that sender has been given 100 ugrug
    suite
        .query_balance(&accounts["sender"], DENOM.clone())
        .should_succeed_and_equal(Uint128::new(100));
    suite
        .query_balance(&accounts["receiver"], DENOM.clone())
        .should_succeed_and_equal(Uint128::ZERO);

    // Sender sends 70 ugrug to the receiver across multiple messages
    suite
        .send_messages(&mut accounts["sender"], vec![
            Message::Transfer {
                to,
                coins: Coins::one(DENOM.clone(), 10).unwrap(),
            },
            Message::Transfer {
                to,
                coins: Coins::one(DENOM.clone(), 15).unwrap(),
            },
            Message::Transfer {
                to,
                coins: Coins::one(DENOM.clone(), 20).unwrap(),
            },
            Message::Transfer {
                to,
                coins: Coins::one(DENOM.clone(), 25).unwrap(),
            },
        ])
        .should_succeed();

    // Check balances again
    suite
        .query_balance(&accounts["sender"], DENOM.clone())
        .should_succeed_and_equal(Uint128::new(30));
    suite
        .query_balance(&accounts["receiver"], DENOM.clone())
        .should_succeed_and_equal(Uint128::new(70));
}
