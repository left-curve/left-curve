use {
    dango_genesis::Contracts,
    dango_testing::{setup_test_naive, Accounts, TestSuite},
    dango_types::{
        account::single,
        account_factory::AccountParams,
        config::AppConfig,
        lending::{
            self, CollateralPower, MarketUpdates, QueryDebtRequest, QueryDebtsRequest,
            QueryMarketsRequest, NAMESPACE, SUBNAMESPACE,
        },
        oracle::{self, PythId},
        token_factory,
    },
    grug::{
        btree_map, Addressable, Binary, Coin, Coins, Denom, HashExt, JsonSerExt, Message,
        MsgConfigure, MsgTransfer, NonEmpty, NumberConst, ResultExt, Udec128, Uint128,
    },
    grug_app::NaiveProposalPreparer,
    grug_vm_rust::VmError,
    std::{str::FromStr, sync::LazyLock},
};

static ATOM: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("uatom").unwrap());
static _OSMO: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("uosmo").unwrap());
static USDC: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("uusdc").unwrap());

/// The Pyth ID for USDC.
pub const USDC_USD_ID: &str = "0xeaa020c61cc479712813461ce153894a96a6c00b21ed0cfc2798d1f9a9e9c94a";

/// An example Pyth VAA for an USDC price feed.
/// - id: **eaa020c61cc479712813461ce153894a96a6c00b21ed0cfc2798d1f9a9e9c94a**
/// - price: **100000966**
/// - ema_price: **99999889**
/// - expo: **-8**
/// - publish_time: **1730802926**
const USDC_VAA: &str = "UE5BVQEAAAADuAEAAAAEDQOoMTxJ5BWLUCMy94ZlQ6qBjQEzA/+ZpDKw9AGFXXSyQF2eIKCGN6cNh1f/jzNSYOf15Yk2CRvOtMc7LqzdG7NpAQSNSaXe+ZOZU4+kxAgG74ZwDUuFmTPlElG90sIMNXfFmS6WJrbTBBQNWFL2gUKpdpEp5z/wUwJo/TzB9lHDnq2vAAbYj1fi3S3mzyOvZAPbe5Qy2/L/oQdLW4FPXTVcNxjMl1m0VLYRonpvIO4/S21ovvsefil9l8R3tYNG879aE2LMAQicgal5v2vVqicVvzE2J1vhg61mEvUKKhiZhzzWo8naRgQfuvKVk3257QhmGaDaAYWxU4MJ7goFUBPbBww9gk53AQpxhRMcpv+qmFMHZCdvoWwF4I/x230bO9VOQXie1tLSf25E62lWTAdYiyrh+h/ny7GA1aDLDZYwEzT6fXUPPlg/AAuQHWuf7TcUkOuIeVisiiI5XINdK8NFu36IacZjf0okOT9dApIx4sLAReROml2hs75v4a1K8SlLB3JdQkQLMoUDAQxZDK7Rh3UBSbjTrBKe+c+5lvT6ZgP26SOqF0F26xJIqwn29C8ZzCKkDgBNzx7GbA4bwL1tNNbv6NSxyx+72AlQAQ2+4nnWuPFUrn5dJJRD5VO6CYNu42Mx4XialbPJ6Lbp3gewVGOIIiU69PyeCxX6/Q/qO99Qtc+QlDGcyjmCwQP1AA7IcDlMiDVc4wEhkfCVRxCr//C3pGZsnxZguQr0MYaSnwGQ/FzJhBsU4knRtTZgUUm3rlcwNWDAJlp5MnNcPuYpAQ91tfYjBU0lRYDoYV/00L+RgJ66vx4P4T3R3x1MuDMAalgVHg43JfcUBGytMHWSbJr/24jMWMsEPMqwBuzPvba5ABDyPKTil7cKBdhyJhTJPYNS0V9JLbS6QLPCThaTyapMMW5BQfB07Q21fXnDNZE/FSoS4JxRiKcViiwRQ4lcLE/UARHcd8PSiHsEilgDjWOH/hvpaQ+Iza+rrBithaw+nJIIdClnizW0DqO2lVx0DlERwF8C9hL3hatj888kVWzwtj6RARJoRZtdYKzWJX8KzJvlOcOBxjjiCSyfo3qLfoLHIw2rJwT7HRxqg1wXswDjq2NVjms9jz24dRIEKM0dxfEP6OckAGcp9O4AAAAAABrhAfrtrFhR4yubI7X5QRqMK6xKrj7U3XuBHdGnLqSqcQAAAAAFb/IiAUFVV1YAAAAAAAp8vHgAACcQO03kFK+kZ552XKseu11fj2cpvpUBAFUA6qAgxhzEeXEoE0Yc4VOJSpamwAsh7Qz8J5jR+anpyUoAAAAABfXkxgAAAAAAAPrQ////+AAAAABnKfTuAAAAAGcp9O4AAAAABfXgkQAAAAAAAQTcCsjx5ZH7wLv7N+2Vzze0aT71EUmuA4n/zf/zQdrI6za/FR4xTLzViierrotGyMoKwkcBs++77xpXHT1p3YXWRMQCLxEONHC/rFMy+rS7i7XohTAftvazeHYjyF6a2rZNmf+KdZS2umZMH9qPKRD3USxGDnXfQMg9mgD6HwJnHiPgaublP56r5AqPcI1tyXKMfF10MWvyxkvJbXFUuYkzW0Pi03Asu75UoUT4XeKBXfvF+EL0NmKGNrmXDYH9NpT5H6pKDeS0JDCZ";

/// Feeds the oracle contract a price for USDC
fn feed_oracle_usdc_price(
    suite: &mut TestSuite<NaiveProposalPreparer>,
    accounts: &mut Accounts,
    contracts: &Contracts,
) {
    let id = PythId::from_str(USDC_USD_ID).unwrap();
    let precision = 6;
    let usdc_denom = Denom::from_str("uusdc").unwrap();

    // Register price source
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &dango_types::oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                usdc_denom.clone() => dango_types::oracle::PriceSource::Pyth { id, precision }
            }),
            Coins::default(),
        )
        .should_succeed();

    // Push price
    {
        suite
            .execute(
                &mut accounts.owner,
                contracts.oracle,
                &oracle::ExecuteMsg::FeedPrices(NonEmpty::new_unchecked(vec![Binary::from_str(
                    USDC_VAA,
                )
                .unwrap()])),
                Coins::default(),
            )
            .should_succeed();

        let current_price = suite
            .query_wasm_smart(contracts.oracle, dango_types::oracle::QueryPriceRequest {
                denom: usdc_denom.clone(),
            })
            .unwrap();

        assert_eq!(
            current_price.humanized_price,
            Udec128::from_str("1.00000966").unwrap()
        );

        assert_eq!(
            current_price.humanized_ema,
            Udec128::from_str("0.99999889").unwrap()
        );

        assert_eq!(current_price.precision(), precision);

        assert_eq!(current_price.timestamp, 1730802926);
    }
}

#[test]
fn cant_transfer_to_lending() {
    let (mut suite, mut accounts, _codes, contracts) = setup_test_naive();

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
    let (mut suite, mut accounts, _codes, contracts) = setup_test_naive();

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

fn set_collateral_power(
    suite: &mut TestSuite<NaiveProposalPreparer>,
    accounts: &mut Accounts,
    denom: Denom,
    power: CollateralPower,
) {
    // Get old config
    let mut config: AppConfig = suite.query_app_config().unwrap();

    // Update collateral power
    config.lending.collateral_powers.insert(denom, power);

    // Set new config
    suite
        .send_message(
            &mut accounts.owner,
            Message::Configure(MsgConfigure {
                new_app_cfg: Some(config.to_json_value().unwrap()),
                new_cfg: None,
            }),
        )
        .should_succeed();

    // Ensure config was updated.
    suite
        .query_app_config::<AppConfig>()
        .should_succeed_and_equal(config.clone());
}

#[test]
fn set_collateral_power_works() {
    let (mut suite, mut accounts, _codes, _contracts) = setup_test_naive();

    set_collateral_power(
        &mut suite,
        &mut accounts,
        USDC.clone(),
        CollateralPower::new(Udec128::new_percent(80)).unwrap(),
    );
}

#[test]
fn deposit_works() {
    let (mut suite, mut accounts, _codes, contracts) = setup_test_naive();

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
    let (mut suite, mut accounts, _codes, contracts) = setup_test_naive();

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
    let (mut suite, mut accounts, _codes, contracts) = setup_test_naive();

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
fn cant_borrow_if_no_collateral() {
    let (mut suite, mut accounts, codes, contracts) = setup_test_naive();

    feed_oracle_usdc_price(&mut suite, &mut accounts, &contracts);

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

    // Deposit some USDC into the lending pool
    suite
        .execute(
            &mut accounts.relayer,
            contracts.lending,
            &lending::ExecuteMsg::Deposit {},
            Coins::one(USDC.clone(), 100).unwrap(),
        )
        .should_succeed();

    // Try to borrow without collateral, should fail
    suite
        .execute(
            &mut margin_account,
            contracts.lending,
            &lending::ExecuteMsg::Borrow(Coins::one(USDC.clone(), 100).unwrap()),
            Coins::new(),
        )
        .should_fail_with_error("the account has no collateral");
}

#[test]
fn cant_borrow_if_undercollateralized() {
    let (mut suite, mut accounts, codes, contracts) = setup_test_naive();

    feed_oracle_usdc_price(&mut suite, &mut accounts, &contracts);

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

    // Deposit some USDC into the lending pool
    suite
        .execute(
            &mut accounts.relayer,
            contracts.lending,
            &lending::ExecuteMsg::Deposit {},
            Coins::one(USDC.clone(), 100).unwrap(),
        )
        .should_succeed();

    // Whitelist USDC as collateral at 90% power
    set_collateral_power(
        &mut suite,
        &mut accounts,
        USDC.clone(),
        CollateralPower::new(Udec128::new_percent(90)).unwrap(),
    );

    // Try to borrow, should fail
    suite
        .execute(
            &mut margin_account,
            contracts.lending,
            &lending::ExecuteMsg::Borrow(Coins::one(USDC.clone(), 100).unwrap()),
            Coins::new(),
        )
        .should_fail_with_error("the action would make the account undercollateralized");
}

#[test]
fn borrowing_works() {
    let (mut suite, mut accounts, codes, contracts) = setup_test_naive();

    feed_oracle_usdc_price(&mut suite, &mut accounts, &contracts);

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

    // Whitelist USDC as collateral at 100% power
    set_collateral_power(
        &mut suite,
        &mut accounts,
        USDC.clone(),
        CollateralPower::new(Udec128::new_percent(100)).unwrap(),
    );

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
    let (mut suite, mut accounts, _, contracts) = setup_test_naive();

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

#[test]
fn all_coins_refunded_if_repaying_when_no_debts() {
    let (mut suite, mut accounts, codes, contracts) = setup_test_naive();

    feed_oracle_usdc_price(&mut suite, &mut accounts, &contracts);

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

    // Send some USDC to the margin account
    suite
        .transfer(
            &mut accounts.relayer,
            margin_account.address(),
            Coins::one(USDC.clone(), 100).unwrap(),
        )
        .should_succeed();

    // Whitelist USDC as collateral
    set_collateral_power(
        &mut suite,
        &mut accounts,
        USDC.clone(),
        CollateralPower::new(Udec128::new_percent(100)).unwrap(),
    );

    // Try to repay, should succeed
    suite
        .execute(
            &mut margin_account,
            contracts.lending,
            &lending::ExecuteMsg::Repay {},
            Coins::one(USDC.clone(), 100).unwrap(),
        )
        .should_succeed();

    // Check that the excess is refunded
    suite
        .query_balance(&margin_account.address(), USDC.clone())
        .should_succeed_and_equal(Uint128::new(100));
}

#[test]
fn excess_refunded_when_repaying_more_than_debts() {
    let (mut suite, mut accounts, codes, contracts) = setup_test_naive();

    feed_oracle_usdc_price(&mut suite, &mut accounts, &contracts);

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

    // Send some USDC to the margin account
    suite
        .transfer(
            &mut accounts.relayer,
            margin_account.address(),
            Coins::one(USDC.clone(), 100).unwrap(),
        )
        .should_succeed();

    // Whitelist USDC as collateral
    set_collateral_power(
        &mut suite,
        &mut accounts,
        USDC.clone(),
        CollateralPower::new(Udec128::new_percent(100)).unwrap(),
    );

    // Deposit some USDC
    suite
        .execute(
            &mut accounts.relayer,
            contracts.lending,
            &lending::ExecuteMsg::Deposit {},
            Coins::one(USDC.clone(), 100).unwrap(),
        )
        .should_succeed();

    // Borrow some USDC
    suite
        .execute(
            &mut margin_account,
            contracts.lending,
            &lending::ExecuteMsg::Borrow(Coins::one(USDC.clone(), 50).unwrap()),
            Coins::new(),
        )
        .should_succeed();

    // Try to repay more than the debts, should succeed
    suite
        .execute(
            &mut margin_account,
            contracts.lending,
            &lending::ExecuteMsg::Repay {},
            Coins::one(USDC.clone(), 100).unwrap(),
        )
        .should_succeed();

    // Check that the excess is refunded
    suite
        .query_balance(&margin_account.address(), USDC.clone())
        .should_succeed_and_equal(Uint128::new(100));
}

#[test]
fn repay_works() {
    let (mut suite, mut accounts, codes, contracts) = setup_test_naive();

    feed_oracle_usdc_price(&mut suite, &mut accounts, &contracts);

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

    // Send some USDC to the margin account
    suite
        .transfer(
            &mut accounts.relayer,
            margin_account.address(),
            Coins::one(USDC.clone(), 100).unwrap(),
        )
        .should_succeed();

    // Whitelist USDC as collateral
    set_collateral_power(
        &mut suite,
        &mut accounts,
        USDC.clone(),
        CollateralPower::new(Udec128::new_percent(100)).unwrap(),
    );

    // Deposit some USDC
    suite
        .execute(
            &mut accounts.relayer,
            contracts.lending,
            &lending::ExecuteMsg::Deposit {},
            Coins::one(USDC.clone(), 100).unwrap(),
        )
        .should_succeed();

    // Borrow some USDC
    suite
        .execute(
            &mut margin_account,
            contracts.lending,
            &lending::ExecuteMsg::Borrow(Coins::one(USDC.clone(), 100).unwrap()),
            Coins::new(),
        )
        .should_succeed();

    // Try to repay, should succeed
    suite
        .execute(
            &mut margin_account,
            contracts.lending,
            &lending::ExecuteMsg::Repay {},
            Coins::one(USDC.clone(), 100).unwrap(),
        )
        .should_succeed();
}
