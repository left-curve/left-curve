use {
    dango_testing::setup_test,
    grug::{Coins, Denom, Message, MsgTransfer},
    std::{str::FromStr, sync::LazyLock},
};

static _ATOM: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("uatom").unwrap());
static _OSMO: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("uosmo").unwrap());
static USDC: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("uusdc").unwrap());

#[test]
fn cant_transfer_to_lending_pool() {
    let (mut suite, mut accounts, _codes, contracts) = setup_test();

    let res = suite
        .send_message(
            &mut accounts.relayer,
            Message::Transfer(MsgTransfer {
                to: contracts.lending_pool,
                coins: Coins::one(USDC.clone(), 123).unwrap(),
            }),
        )
        .result;

    assert!(res.is_err_and(|err| err
        .to_string()
        .contains("Can't send tokens to this contract")));
}
