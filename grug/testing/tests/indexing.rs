use {
    grug_testing::TestBuilder,
    grug_types::{Coins, Denom, Message},
    std::str::FromStr,
};

#[test]
fn index_block() {
    let denom = Denom::from_str("ugrug").unwrap();
    let (mut suite, mut accounts) = TestBuilder::new()
        .add_account("owner", Coins::new())
        .add_account("sender", Coins::one(denom.clone(), 30_000).unwrap())
        .set_owner("owner")
        .build();

    let to = accounts["sender"].address;

    let _outcome = suite.send_message_with_gas(
        &mut accounts["sender"],
        2000,
        Message::transfer(to, Coins::new()).unwrap(),
    );
}
