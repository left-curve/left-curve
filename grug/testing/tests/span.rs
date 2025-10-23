use {
    grug_testing::TestBuilder,
    grug_types::{Coin, Coins, Message, NonEmpty, ResultExt},
    tracing::Level,
};

#[test]
fn span() {
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("failed to set global tracing subscriber");

    let (mut suite, mut accounts) = TestBuilder::new()
        .add_account("owner", Coin::new("usdc", 100_000).unwrap())
        .add_account("receiver", Coins::new())
        .set_owner("owner")
        .set_tracing_level(None)
        .build();

    let receiver_addr = accounts["receiver"].address;

    suite
        .send_messages(
            &mut accounts["owner"],
            NonEmpty::new_unchecked(vec![
                Message::transfer(receiver_addr, Coins::one("usdc", 100).unwrap()).unwrap(),
                Message::transfer(receiver_addr, Coins::one("ugrug", 100).unwrap()).unwrap(),
            ]),
        )
        .should_fail();
}
