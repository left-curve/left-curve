use {
    dango_testing::setup_test,
    dango_types::{
        account::single,
        account_factory::AccountParams,
        lending::{
            self, MarketUpdates, QueryDebtRequest, QueryDebtsRequest, QueryMarketsRequest,
            NAMESPACE, SUBNAMESPACE,
        },
        token_factory,
    },
    grug::{
        btree_map, Addressable, Coin, Coins, Denom, HashExt, Message, MsgTransfer, NumberConst,
        ResultExt, Uint128,
    },
    grug_vm_rust::VmError,
    std::{str::FromStr, sync::LazyLock},
};

static ATOM: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("uatom").unwrap());
static _OSMO: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("uosmo").unwrap());
static USDC: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("uusdc").unwrap());

#[test]
fn cant_transfer_to_lending() {
    let (mut suite, mut accounts, _codes, contracts) = setup_test();

    suite
        .send_message(
            &mut accounts.relayer,
            Message::Transfer(MsgTransfer {
                to: contracts.lending,
                coins: Coins::one(USDC.clone(), 123).unwrap(),
            }),
        )
        .should_fail_with_error(VmError::function_not_found("receive"));
}

#[test]
fn update_markets_works() {
    let (mut suite, mut accounts, _codes, contracts) = setup_test();

    // Ensure USDC market already exists.
    suite
        .query_wasm_smart(contracts.lending, QueryMarketsRequest {
            limit: None,
            start_after: None,
        })
        .should_succeed_and(|markets| markets.contains_key(&USDC));

    // Try to update markets from non-owner, should fail.
    suite
        .execute(
            &mut accounts.relayer,
            contracts.lending,
            &lending::ExecuteMsg::UpdateMarkets(btree_map! {}),
            Coins::new(),
        )
        .should_fail_with_error("Only the owner can whitelist denoms");

    // Whitelist ATOM from owner, should succeed.
    suite
        .execute(
            &mut accounts.owner,
            contracts.lending,
            &lending::ExecuteMsg::UpdateMarkets(btree_map! {
                ATOM.clone() => MarketUpdates {},
            }),
            Coins::new(),
        )
        .should_succeed();

    // Ensure ATOM market now exists.
    suite
        .query_wasm_smart(contracts.lending, QueryMarketsRequest {
            limit: None,
            start_after: None,
        })
        .should_succeed_and(|markets| markets.contains_key(&ATOM));
}

#[test]
fn deposit_works() {
    let (mut suite, mut accounts, _codes, contracts) = setup_test();

    let lp_denom = USDC.prepend(&[&NAMESPACE, &SUBNAMESPACE]).unwrap();

    let balance_before = suite
        .query_balance(&accounts.relayer, USDC.clone())
        .unwrap();

    suite
        .execute(
            &mut accounts.relayer,
            contracts.lending,
            &lending::ExecuteMsg::Deposit {},
            Coins::one(USDC.clone(), 123).unwrap(),
        )
        .should_succeed();

    // Ensure balance was deducted from depositor.
    suite
        .query_balance(&accounts.relayer, USDC.clone())
        .should_succeed_and_equal(balance_before - Uint128::new(123));

    // Ensure LP token was minted to recipient.
    suite
        .query_balance(&accounts.relayer, lp_denom)
        .should_succeed_and_equal(Uint128::new(123));
}

#[test]
fn withdraw_works() {
    let (mut suite, mut accounts, _codes, contracts) = setup_test();

    let lp_denom = USDC.prepend(&[&NAMESPACE, &SUBNAMESPACE]).unwrap();

    // First deposit
    suite
        .execute(
            &mut accounts.relayer,
            contracts.lending,
            &lending::ExecuteMsg::Deposit {},
            Coins::one(USDC.clone(), 123).unwrap(),
        )
        .should_succeed();

    suite
        .query_balance(&accounts.relayer.address(), lp_denom.clone())
        .should_succeed_and_equal(Uint128::new(123));

    let balance_before = suite
        .query_balance(&accounts.relayer, USDC.clone())
        .unwrap();

    // Now withdraw
    suite
        .execute(
            &mut accounts.relayer,
            contracts.lending,
            &lending::ExecuteMsg::Withdraw {},
            Coins::one(lp_denom.clone(), 123).unwrap(),
        )
        .should_succeed();

    // Ensure LP token was burned from withdrawer.
    suite
        .query_balance(&accounts.relayer.address(), lp_denom)
        .should_succeed_and_equal(Uint128::new(0));

    // Ensure balance was added to recipient.
    suite
        .query_balance(&accounts.relayer, USDC.clone())
        .should_succeed_and_equal(balance_before + Uint128::new(123));
}

#[test]
fn non_margin_accounts_cant_borrow() {
    let (mut suite, mut accounts, _codes, contracts) = setup_test();

    suite
        .execute(
            &mut accounts.relayer,
            contracts.lending,
            &lending::ExecuteMsg::Borrow(Coins::new()),
            Coins::new(),
        )
        .should_fail_with_error("Only margin accounts can borrow");
}

#[test]
fn borrowing_works() {
    let (mut suite, mut accounts, codes, contracts) = setup_test();

    // Create a margin account.
    let mut margin_account = accounts
        .relayer
        .register_new_account(
            &mut suite,
            contracts.account_factory,
            codes.account_margin.to_bytes().hash256(),
            AccountParams::Margin(single::Params {
                owner: accounts.relayer.username.clone(),
            }),
            Coins::new(),
        )
        .unwrap();

    // Try to borrow from the margin account, should succeed fail as no coins are deposited
    suite
        .execute(
            &mut margin_account,
            contracts.lending,
            &lending::ExecuteMsg::Borrow(Coins::one(USDC.clone(), 100).unwrap()),
            Coins::new(),
        )
        .should_fail_with_error("subtraction overflow: 0 - 100");

    // Deposit some USDC
    suite
        .execute(
            &mut accounts.relayer,
            contracts.lending,
            &lending::ExecuteMsg::Deposit {},
            Coins::one(USDC.clone(), 100).unwrap(),
        )
        .should_succeed();

    // Try to borrow again, should succeed
    suite
        .execute(
            &mut margin_account,
            contracts.lending,
            &lending::ExecuteMsg::Borrow(Coins::one(USDC.clone(), 100).unwrap()),
            Coins::new(),
        )
        .should_succeed();

    // Confirm the margin account has the borrowed coins
    suite
        .query_balance(&margin_account.address(), USDC.clone())
        .should_succeed_and_equal(Uint128::new(100));

    // Confirm that the lending pool has the liability
    suite
        .query_wasm_smart(contracts.lending, QueryDebtRequest {
            account: margin_account.address(),
        })
        .should_succeed_and_equal(Coins::one(USDC.clone(), 100).unwrap());

    suite
        .query_wasm_smart(contracts.lending, QueryDebtsRequest {
            limit: None,
            start_after: None,
        })
        .should_succeed_and_equal(btree_map! {
            margin_account.address() => Coins::one(USDC.clone(), 100).unwrap(),
        });
}

#[test]
fn composite_denom() {
    let (mut suite, mut accounts, _, contracts) = setup_test();

    let fee_token_creation = Coin::new("uusdc", 10_000_000_u128).unwrap();
    let amount: Uint128 = 100_000.into();
    let owner_addr = accounts.owner.address();

    // Create a new token with token_factory
    suite
        .execute(
            &mut accounts.owner,
            contracts.token_factory,
            &token_factory::ExecuteMsg::Create {
                subdenom: Denom::from_str("foo").unwrap(),
                username: None,
                admin: None,
            },
            fee_token_creation,
        )
        .should_succeed();

    let denom = Denom::from_str(&format!("factory/{}/foo", owner_addr)).unwrap();

    // Register the denom in the lending
    suite
        .execute(
            &mut accounts.owner,
            contracts.lending,
            &lending::ExecuteMsg::UpdateMarkets(btree_map! {
                denom.clone() => MarketUpdates {},
            }),
            Coins::default(),
        )
        .should_succeed();

    // Mint some tokens
    suite
        .execute(
            &mut accounts.owner,
            contracts.token_factory,
            &token_factory::ExecuteMsg::Mint {
                denom: denom.clone(),
                to: owner_addr,
                amount,
            },
            Coins::default(),
        )
        .should_succeed();

    // Deposit some tokens
    suite
        .execute(
            &mut accounts.owner,
            contracts.lending,
            &lending::ExecuteMsg::Deposit {},
            Coins::one(denom.clone(), amount).unwrap(),
        )
        .should_succeed();

    let lp_token = denom.prepend(&[&NAMESPACE, &SUBNAMESPACE]).unwrap();

    // check if lp_token is minted
    suite
        .query_balance(&accounts.owner.address(), lp_token.clone())
        .should_succeed_and_equal(amount);

    // withdraw
    suite
        .execute(
            &mut accounts.owner,
            contracts.lending,
            &lending::ExecuteMsg::Withdraw {},
            Coins::one(lp_token.clone(), amount).unwrap(),
        )
        .should_succeed();

    // check if lp_token is burned
    suite
        .query_balance(&accounts.owner.address(), lp_token)
        .should_succeed_and_equal(Uint128::ZERO);

    // check if lp_token is burned
    suite
        .query_balance(&accounts.owner.address(), denom)
        .should_succeed_and_equal(amount);
}
