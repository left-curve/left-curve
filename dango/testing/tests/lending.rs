use {
    dango_genesis::Contracts,
    dango_testing::{TestAccount, TestAccounts, TestSuite, setup_test_naive},
    dango_types::{
        account::{margin::CollateralPower, single},
        account_factory::AccountParams,
        config::AppConfig,
        constants::{ATOM_DENOM, USDC_DENOM},
        lending::{
            self, InterestRateModel, MarketUpdates, NAMESPACE, QueryDebtRequest, QueryDebtsRequest,
            QueryMarketRequest, QueryMarketsRequest, QueryPreviewWithdrawRequest, SECONDS_PER_YEAR,
            SUBNAMESPACE,
        },
        oracle::{self, PriceSource},
    },
    grug::{
        Addressable, Binary, Coins, Denom, Duration, JsonSerExt, Message, MsgConfigure,
        MultiplyFraction, NonEmpty, NumberConst, QuerierExt, ResultExt, Sign, Udec128, Udec256,
        Uint128, btree_map, coins,
    },
    grug_app::NaiveProposalPreparer,
    grug_vm_rust::VmError,
    std::str::FromStr,
};

/// An example Pyth VAA for an USDC price feed.
/// - id: **eaa020c61cc479712813461ce153894a96a6c00b21ed0cfc2798d1f9a9e9c94a**
/// - price: **100000966**
/// - ema_price: **99999889**
/// - expo: **-8**
/// - publish_time: **1730802926**
const USDC_VAA: &str = "UE5BVQEAAAADuAEAAAAEDQOoMTxJ5BWLUCMy94ZlQ6qBjQEzA/+ZpDKw9AGFXXSyQF2eIKCGN6cNh1f/jzNSYOf15Yk2CRvOtMc7LqzdG7NpAQSNSaXe+ZOZU4+kxAgG74ZwDUuFmTPlElG90sIMNXfFmS6WJrbTBBQNWFL2gUKpdpEp5z/wUwJo/TzB9lHDnq2vAAbYj1fi3S3mzyOvZAPbe5Qy2/L/oQdLW4FPXTVcNxjMl1m0VLYRonpvIO4/S21ovvsefil9l8R3tYNG879aE2LMAQicgal5v2vVqicVvzE2J1vhg61mEvUKKhiZhzzWo8naRgQfuvKVk3257QhmGaDaAYWxU4MJ7goFUBPbBww9gk53AQpxhRMcpv+qmFMHZCdvoWwF4I/x230bO9VOQXie1tLSf25E62lWTAdYiyrh+h/ny7GA1aDLDZYwEzT6fXUPPlg/AAuQHWuf7TcUkOuIeVisiiI5XINdK8NFu36IacZjf0okOT9dApIx4sLAReROml2hs75v4a1K8SlLB3JdQkQLMoUDAQxZDK7Rh3UBSbjTrBKe+c+5lvT6ZgP26SOqF0F26xJIqwn29C8ZzCKkDgBNzx7GbA4bwL1tNNbv6NSxyx+72AlQAQ2+4nnWuPFUrn5dJJRD5VO6CYNu42Mx4XialbPJ6Lbp3gewVGOIIiU69PyeCxX6/Q/qO99Qtc+QlDGcyjmCwQP1AA7IcDlMiDVc4wEhkfCVRxCr//C3pGZsnxZguQr0MYaSnwGQ/FzJhBsU4knRtTZgUUm3rlcwNWDAJlp5MnNcPuYpAQ91tfYjBU0lRYDoYV/00L+RgJ66vx4P4T3R3x1MuDMAalgVHg43JfcUBGytMHWSbJr/24jMWMsEPMqwBuzPvba5ABDyPKTil7cKBdhyJhTJPYNS0V9JLbS6QLPCThaTyapMMW5BQfB07Q21fXnDNZE/FSoS4JxRiKcViiwRQ4lcLE/UARHcd8PSiHsEilgDjWOH/hvpaQ+Iza+rrBithaw+nJIIdClnizW0DqO2lVx0DlERwF8C9hL3hatj888kVWzwtj6RARJoRZtdYKzWJX8KzJvlOcOBxjjiCSyfo3qLfoLHIw2rJwT7HRxqg1wXswDjq2NVjms9jz24dRIEKM0dxfEP6OckAGcp9O4AAAAAABrhAfrtrFhR4yubI7X5QRqMK6xKrj7U3XuBHdGnLqSqcQAAAAAFb/IiAUFVV1YAAAAAAAp8vHgAACcQO03kFK+kZ552XKseu11fj2cpvpUBAFUA6qAgxhzEeXEoE0Yc4VOJSpamwAsh7Qz8J5jR+anpyUoAAAAABfXkxgAAAAAAAPrQ////+AAAAABnKfTuAAAAAGcp9O4AAAAABfXgkQAAAAAAAQTcCsjx5ZH7wLv7N+2Vzze0aT71EUmuA4n/zf/zQdrI6za/FR4xTLzViierrotGyMoKwkcBs++77xpXHT1p3YXWRMQCLxEONHC/rFMy+rS7i7XohTAftvazeHYjyF6a2rZNmf+KdZS2umZMH9qPKRD3USxGDnXfQMg9mgD6HwJnHiPgaublP56r5AqPcI1tyXKMfF10MWvyxkvJbXFUuYkzW0Pi03Asu75UoUT4XeKBXfvF+EL0NmKGNrmXDYH9NpT5H6pKDeS0JDCZ";

/// Asserts that two values are within one of each other.
fn assert_eq_or_one_off(a: impl Into<Uint128>, b: impl Into<Uint128>) {
    let a = a.into();
    let b = b.into();

    let diff = if a > b {
        a - b
    } else {
        b - a
    };

    assert!(diff <= Uint128::ONE);
}

/// Feeds the oracle contract a price for USDC
fn feed_oracle_usdc_price(
    suite: &mut TestSuite<NaiveProposalPreparer>,
    accounts: &mut TestAccounts,
    contracts: &Contracts,
) {
    let precision = 6;

    // Push price
    {
        suite
            .execute(
                &mut accounts.owner,
                contracts.oracle,
                &oracle::ExecuteMsg::FeedPrices(NonEmpty::new_unchecked(vec![
                    Binary::from_str(USDC_VAA).unwrap(),
                ])),
                Coins::default(),
            )
            .should_succeed();

        let current_price = suite
            .query_wasm_smart(contracts.oracle, dango_types::oracle::QueryPriceRequest {
                denom: USDC_DENOM.clone(),
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
            &mut accounts.user1,
            Message::transfer(contracts.lending, coins! { USDC_DENOM.clone() => 123 }).unwrap(),
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
        .should_succeed_and(|markets| markets.contains_key(&USDC_DENOM));

    // Try to update markets from non-owner, should fail.
    suite
        .execute(
            &mut accounts.user1,
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
                ATOM_DENOM.clone() => MarketUpdates {
                    interest_rate_model: Some(InterestRateModel::default()),
                },
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
        .should_succeed_and(|markets| markets.contains_key(&ATOM_DENOM));
}

#[test]
fn indexes_are_updated_when_interest_rate_model_is_updated() {
    let (mut suite, mut accounts, _codes, contracts) = setup_test_naive();

    // Ensure USDC market already exists.
    suite
        .query_wasm_smart(contracts.lending, QueryMarketsRequest {
            limit: None,
            start_after: None,
        })
        .should_succeed_and(|markets| markets.contains_key(&USDC_DENOM));

    // deposit some USDC
    suite
        .execute(
            &mut accounts.user1,
            contracts.lending,
            &lending::ExecuteMsg::Deposit {},
            Coins::one(USDC_DENOM.clone(), 100).unwrap(),
        )
        .should_succeed();

    // Create a margin account.
    let mut margin_account = accounts
        .user1
        .register_new_account(
            &mut suite,
            contracts.account_factory,
            AccountParams::Margin(single::Params::new(accounts.user1.username.clone())),
            Coins::new(),
        )
        .unwrap();

    // Whitelist USDC as collateral at 100% power
    set_collateral_power(
        &mut suite,
        &mut accounts,
        USDC_DENOM.clone(),
        CollateralPower::new(Udec128::new_percent(100)).unwrap(),
    );

    // Register price source for USDC
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                USDC_DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::ONE,
                    precision: 6,
                    timestamp: 1730802926,
                },
            }),
            Coins::new(),
        )
        .should_succeed();

    // Borrow some USDC
    suite
        .execute(
            &mut margin_account,
            contracts.lending,
            &lending::ExecuteMsg::Borrow(NonEmpty::new_unchecked(
                coins! { USDC_DENOM.clone() => 100 },
            )),
            Coins::new(),
        )
        .should_succeed();

    // Increase time to accrue interest
    suite.increase_time(Duration::from_seconds(60 * 60 * 24)); // 1 day

    // Query the current market for USDC
    let market = suite
        .query_wasm_smart(contracts.lending, QueryMarketRequest {
            denom: USDC_DENOM.clone(),
        })
        .unwrap();

    let old_borrow_index = market.borrow_index;
    let old_supply_index = market.supply_index;

    // Update the interest rate model
    suite
        .execute(
            &mut accounts.owner,
            contracts.lending,
            &lending::ExecuteMsg::UpdateMarkets(btree_map! {
                USDC_DENOM.clone() => MarketUpdates {
                    interest_rate_model: Some(InterestRateModel::default()),
                },
            }),
            Coins::new(),
        )
        .should_succeed();

    // Query the updated market for USDC
    let updated_market = suite
        .query_wasm_smart(contracts.lending, QueryMarketRequest {
            denom: USDC_DENOM.clone(),
        })
        .unwrap();

    assert!(updated_market.borrow_index > old_borrow_index);
    assert!(updated_market.supply_index > old_supply_index);
}

fn set_collateral_power(
    suite: &mut TestSuite<NaiveProposalPreparer>,
    accounts: &mut TestAccounts,
    denom: Denom,
    power: CollateralPower,
) {
    // Get old config
    let mut config: AppConfig = suite.query_app_config().unwrap();

    // Update collateral power
    config.collateral_powers.insert(denom, power);

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
        USDC_DENOM.clone(),
        CollateralPower::new(Udec128::new_percent(80)).unwrap(),
    );
}

#[test]
fn deposit_works() {
    let (mut suite, mut accounts, _codes, contracts) = setup_test_naive();

    let lp_denom = USDC_DENOM.prepend(&[&NAMESPACE, &SUBNAMESPACE]).unwrap();
    let balance_before = suite
        .query_balance(&accounts.user1, USDC_DENOM.clone())
        .unwrap();

    suite
        .execute(
            &mut accounts.user1,
            contracts.lending,
            &lending::ExecuteMsg::Deposit {},
            Coins::one(USDC_DENOM.clone(), 123).unwrap(),
        )
        .should_succeed();

    // Ensure balance was deducted from depositor.
    suite
        .query_balance(&accounts.user1, USDC_DENOM.clone())
        .should_succeed_and_equal(balance_before - Uint128::new(123));

    // Ensure LP token was minted to recipient.
    suite
        .query_balance(&accounts.user1, lp_denom)
        .should_succeed_and_equal(Uint128::new(123));
}

#[test]
fn withdraw_works() {
    let (mut suite, mut accounts, _codes, contracts) = setup_test_naive();

    let lp_denom = USDC_DENOM.prepend(&[&NAMESPACE, &SUBNAMESPACE]).unwrap();

    // First deposit
    suite
        .execute(
            &mut accounts.user1,
            contracts.lending,
            &lending::ExecuteMsg::Deposit {},
            Coins::one(USDC_DENOM.clone(), 123).unwrap(),
        )
        .should_succeed();

    suite
        .query_balance(&accounts.user1.address(), lp_denom.clone())
        .should_succeed_and_equal(Uint128::new(123));

    let balance_before = suite
        .query_balance(&accounts.user1, USDC_DENOM.clone())
        .unwrap();

    // Now withdraw
    suite
        .execute(
            &mut accounts.user1,
            contracts.lending,
            &lending::ExecuteMsg::Withdraw {},
            Coins::one(lp_denom.clone(), 123).unwrap(),
        )
        .should_succeed();

    // Ensure LP token was burned from withdrawer.
    suite
        .query_balance(&accounts.user1.address(), lp_denom)
        .should_succeed_and_equal(Uint128::new(0));

    // Ensure balance was added to recipient.
    suite
        .query_balance(&accounts.user1, USDC_DENOM.clone())
        .should_succeed_and_equal(balance_before + Uint128::new(123));
}

#[test]
fn non_margin_accounts_cant_borrow() {
    let (mut suite, mut accounts, _codes, contracts) = setup_test_naive();

    suite
        .execute(
            &mut accounts.user1,
            contracts.lending,
            &lending::ExecuteMsg::Borrow(NonEmpty::new_unchecked(
                coins! { USDC_DENOM.clone() => 100 },
            )),
            Coins::new(),
        )
        .should_fail_with_error("only margin accounts can borrow");
}

#[test]
fn cant_borrow_if_no_collateral() {
    let (mut suite, mut accounts, _, contracts) = setup_test_naive();

    feed_oracle_usdc_price(&mut suite, &mut accounts, &contracts);

    // Create a margin account.
    let mut margin_account = accounts
        .user1
        .register_new_account(
            &mut suite,
            contracts.account_factory,
            AccountParams::Margin(single::Params::new(accounts.user1.username.clone())),
            Coins::new(),
        )
        .unwrap();

    // Deposit some USDC into the lending pool
    suite
        .execute(
            &mut accounts.user1,
            contracts.lending,
            &lending::ExecuteMsg::Deposit {},
            Coins::one(USDC_DENOM.clone(), 100).unwrap(),
        )
        .should_succeed();

    // Try to borrow without collateral, should fail
    suite
        .execute(
            &mut margin_account,
            contracts.lending,
            &lending::ExecuteMsg::Borrow(NonEmpty::new_unchecked(
                coins! { USDC_DENOM.clone() => 100 },
            )),
            Coins::new(),
        )
        .should_fail_with_error("this action would make account undercollateralized!");
}

#[test]
fn cant_borrow_if_undercollateralized() {
    let (mut suite, mut accounts, _, contracts) = setup_test_naive();

    feed_oracle_usdc_price(&mut suite, &mut accounts, &contracts);

    // Create a margin account.
    let mut margin_account = accounts
        .user1
        .register_new_account(
            &mut suite,
            contracts.account_factory,
            AccountParams::Margin(single::Params::new(accounts.user1.username.clone())),
            Coins::new(),
        )
        .unwrap();

    // Deposit some USDC into the lending pool
    suite
        .execute(
            &mut accounts.user1,
            contracts.lending,
            &lending::ExecuteMsg::Deposit {},
            Coins::one(USDC_DENOM.clone(), 100).unwrap(),
        )
        .should_succeed();

    // Whitelist USDC as collateral at 90% power
    set_collateral_power(
        &mut suite,
        &mut accounts,
        USDC_DENOM.clone(),
        CollateralPower::new(Udec128::new_percent(90)).unwrap(),
    );

    // Try to borrow, should fail
    suite
        .execute(
            &mut margin_account,
            contracts.lending,
            &lending::ExecuteMsg::Borrow(NonEmpty::new_unchecked(
                coins! { USDC_DENOM.clone() => 100 },
            )),
            Coins::new(),
        )
        .should_fail_with_error("this action would make account undercollateralized!");
}

#[test]
fn borrowing_works() {
    let (mut suite, mut accounts, _, contracts) = setup_test_naive();

    feed_oracle_usdc_price(&mut suite, &mut accounts, &contracts);

    // Create a margin account.
    let mut margin_account = accounts
        .user1
        .register_new_account(
            &mut suite,
            contracts.account_factory,
            AccountParams::Margin(single::Params::new(accounts.user1.username.clone())),
            Coins::new(),
        )
        .unwrap();

    // Try to borrow from the margin account, should succeed fail as no coins are deposited
    suite
        .execute(
            &mut margin_account,
            contracts.lending,
            &lending::ExecuteMsg::Borrow(NonEmpty::new_unchecked(
                coins! { USDC_DENOM.clone() => 100 },
            )),
            Coins::new(),
        )
        .should_fail_with_error("subtraction overflow: 0 - 100");

    // Deposit some USDC
    suite
        .execute(
            &mut accounts.user1,
            contracts.lending,
            &lending::ExecuteMsg::Deposit {},
            Coins::one(USDC_DENOM.clone(), 100).unwrap(),
        )
        .should_succeed();

    // Whitelist USDC as collateral at 100% power
    set_collateral_power(
        &mut suite,
        &mut accounts,
        USDC_DENOM.clone(),
        CollateralPower::new(Udec128::new_percent(100)).unwrap(),
    );

    // Try to borrow again, should succeed
    suite
        .execute(
            &mut margin_account,
            contracts.lending,
            &lending::ExecuteMsg::Borrow(NonEmpty::new_unchecked(
                coins! { USDC_DENOM.clone() => 100 },
            )),
            Coins::new(),
        )
        .should_succeed();

    // Confirm the margin account has the borrowed coins
    suite
        .query_balance(&margin_account.address(), USDC_DENOM.clone())
        .should_succeed_and_equal(Uint128::new(100));

    // Confirm that the lending pool has the liability
    suite
        .query_wasm_smart(contracts.lending, QueryDebtRequest {
            account: margin_account.address(),
        })
        .should_succeed_and_equal(Coins::one(USDC_DENOM.clone(), 100).unwrap());

    suite
        .query_wasm_smart(contracts.lending, QueryDebtsRequest {
            limit: None,
            start_after: None,
        })
        .should_succeed_and_equal(btree_map! {
            margin_account.address() => Coins::one(USDC_DENOM.clone(), 100).unwrap(),
        });
}

#[test]
fn all_coins_refunded_if_repaying_when_no_debts() {
    let (mut suite, mut accounts, _, contracts) = setup_test_naive();

    feed_oracle_usdc_price(&mut suite, &mut accounts, &contracts);

    // Create a margin account.
    let mut margin_account = accounts
        .user1
        .register_new_account(
            &mut suite,
            contracts.account_factory,
            AccountParams::Margin(single::Params::new(accounts.user1.username.clone())),
            Coins::new(),
        )
        .unwrap();

    // Send some USDC to the margin account
    suite
        .transfer(
            &mut accounts.user1,
            margin_account.address(),
            Coins::one(USDC_DENOM.clone(), 100).unwrap(),
        )
        .should_succeed();

    // Whitelist USDC as collateral
    set_collateral_power(
        &mut suite,
        &mut accounts,
        USDC_DENOM.clone(),
        CollateralPower::new(Udec128::new_percent(100)).unwrap(),
    );

    // Try to repay, should succeed
    suite
        .execute(
            &mut margin_account,
            contracts.lending,
            &lending::ExecuteMsg::Repay {},
            Coins::one(USDC_DENOM.clone(), 100).unwrap(),
        )
        .should_succeed();

    // Check that the excess is refunded
    suite
        .query_balance(&margin_account.address(), USDC_DENOM.clone())
        .should_succeed_and_equal(Uint128::new(100));
}

#[test]
fn excess_refunded_when_repaying_more_than_debts() {
    let (mut suite, mut accounts, _, contracts) = setup_test_naive();

    feed_oracle_usdc_price(&mut suite, &mut accounts, &contracts);

    // Create a margin account.
    let mut margin_account = accounts
        .user1
        .register_new_account(
            &mut suite,
            contracts.account_factory,
            AccountParams::Margin(single::Params::new(accounts.user1.username.clone())),
            Coins::new(),
        )
        .unwrap();

    // Send some USDC to the margin account
    suite
        .transfer(
            &mut accounts.user1,
            margin_account.address(),
            Coins::one(USDC_DENOM.clone(), 100).unwrap(),
        )
        .should_succeed();

    // Whitelist USDC as collateral
    set_collateral_power(
        &mut suite,
        &mut accounts,
        USDC_DENOM.clone(),
        CollateralPower::new(Udec128::new_percent(100)).unwrap(),
    );

    // Deposit some USDC
    suite
        .execute(
            &mut accounts.user1,
            contracts.lending,
            &lending::ExecuteMsg::Deposit {},
            Coins::one(USDC_DENOM.clone(), 100).unwrap(),
        )
        .should_succeed();

    // Borrow some USDC
    suite
        .execute(
            &mut margin_account,
            contracts.lending,
            &lending::ExecuteMsg::Borrow(NonEmpty::new_unchecked(
                coins! { USDC_DENOM.clone() => 100 },
            )),
            Coins::new(),
        )
        .should_succeed();

    // Try to repay more than the debts, should succeed
    suite
        .execute(
            &mut margin_account,
            contracts.lending,
            &lending::ExecuteMsg::Repay {},
            Coins::one(USDC_DENOM.clone(), 100).unwrap(),
        )
        .should_succeed();

    // Check that the excess is refunded
    suite
        .query_balance(&margin_account.address(), USDC_DENOM.clone())
        .should_succeed_and_equal(Uint128::new(100));
}

#[test]
fn repay_works() {
    let (mut suite, mut accounts, _, contracts) = setup_test_naive();

    feed_oracle_usdc_price(&mut suite, &mut accounts, &contracts);

    // Create a margin account.
    let mut margin_account = accounts
        .user1
        .register_new_account(
            &mut suite,
            contracts.account_factory,
            AccountParams::Margin(single::Params::new(accounts.user1.username.clone())),
            Coins::new(),
        )
        .unwrap();

    // Send some USDC to the margin account
    suite
        .transfer(
            &mut accounts.user1,
            margin_account.address(),
            Coins::one(USDC_DENOM.clone(), 100).unwrap(),
        )
        .should_succeed();

    // Whitelist USDC as collateral
    set_collateral_power(
        &mut suite,
        &mut accounts,
        USDC_DENOM.clone(),
        CollateralPower::new(Udec128::new_percent(100)).unwrap(),
    );

    // Deposit some USDC
    suite
        .execute(
            &mut accounts.user1,
            contracts.lending,
            &lending::ExecuteMsg::Deposit {},
            Coins::one(USDC_DENOM.clone(), 100).unwrap(),
        )
        .should_succeed();

    // Borrow some USDC
    suite
        .execute(
            &mut margin_account,
            contracts.lending,
            &lending::ExecuteMsg::Borrow(NonEmpty::new_unchecked(
                coins! { USDC_DENOM.clone() => 100 },
            )),
            Coins::new(),
        )
        .should_succeed();

    // Try to repay, should succeed
    suite
        .execute(
            &mut margin_account,
            contracts.lending,
            &lending::ExecuteMsg::Repay {},
            Coins::one(USDC_DENOM.clone(), 100).unwrap(),
        )
        .should_succeed();
}

fn interest_rate_setup() -> (
    TestSuite<NaiveProposalPreparer>,
    TestAccounts,
    Contracts,
    TestAccount,
) {
    let (mut suite, mut accounts, _, contracts) = setup_test_naive();

    feed_oracle_usdc_price(&mut suite, &mut accounts, &contracts);

    // Add borrow/lend market for USDC
    suite
        .execute(
            &mut accounts.owner,
            contracts.lending,
            &lending::ExecuteMsg::UpdateMarkets(btree_map! {
                USDC_DENOM.clone() => MarketUpdates {
                    interest_rate_model: Some(InterestRateModel::default()),
                },
            }),
            Coins::new(),
        )
        .should_succeed();

    // Create a margin account.
    let margin_account = accounts
        .user1
        .register_new_account(
            &mut suite,
            contracts.account_factory,
            AccountParams::Margin(single::Params::new(accounts.user1.username.clone())),
            Coins::new(),
        )
        .unwrap();

    // Send some USDC to the margin account
    suite
        .transfer(
            &mut accounts.owner,
            margin_account.address(),
            Coins::one(USDC_DENOM.clone(), 100_000_000_000).unwrap(),
        )
        .should_succeed();

    // Whitelist USDC as collateral at 100% collateral power
    set_collateral_power(
        &mut suite,
        &mut accounts,
        USDC_DENOM.clone(),
        CollateralPower::new(Udec128::new_percent(100)).unwrap(),
    );

    (suite, accounts, contracts, margin_account)
}

#[test]
fn interest_rate_model_works_multiple_times() {
    let (mut suite, mut accounts, contracts, mut margin_account) = interest_rate_setup();

    let iterations = 3;
    for _ in 0..iterations {
        interest_rate_model_works(
            &mut suite,
            &mut accounts,
            contracts.clone(),
            &mut margin_account,
        );
    }
}

fn interest_rate_model_works(
    suite: &mut TestSuite<NaiveProposalPreparer>,
    accounts: &mut TestAccounts,
    contracts: Contracts,
    margin_account: &mut TestAccount,
) {
    // --- Property 1: Deposit interest rate is zero when no one has borrowed yet ---

    // Query the current interest rate model
    let market = suite
        .query_wasm_smart(contracts.lending, QueryMarketRequest {
            denom: USDC_DENOM.clone(),
        })
        .should_succeed();
    assert_eq!(market.interest_rate_model, InterestRateModel::default());

    // Compute interest rates
    let interest_rate = market
        .interest_rate_model
        .calculate_rates(market.utilization_rate(suite).unwrap());

    // Assert that the supply interest rate is zero (since no one has borrowed yet)
    assert_eq!(interest_rate.deposit_rate, Udec128::ZERO);

    // Deposit some USDC
    let deposit_amount = 1_000_000_000u128;
    suite
        .execute(
            &mut accounts.user1,
            contracts.lending,
            &lending::ExecuteMsg::Deposit {},
            Coins::one(USDC_DENOM.clone(), deposit_amount).unwrap(),
        )
        .should_succeed();

    // Query the users LP token balance
    let lp_denom = USDC_DENOM
        .clone()
        .prepend(&[&NAMESPACE, &SUBNAMESPACE])
        .unwrap();
    let lp_balance = suite
        .query_balance(&accounts.user1.address(), lp_denom.clone())
        .should_succeed();

    // Query how many tokens the user can withdraw with their LP tokens
    let withdraw_amount = suite
        .query_wasm_smart(contracts.lending, QueryPreviewWithdrawRequest {
            lp_tokens: Coins::one(lp_denom.clone(), lp_balance).unwrap(),
        })
        .should_succeed();

    // Check that the withdraw amount is correct
    assert_eq_or_one_off(
        withdraw_amount.amount_of(&USDC_DENOM),
        Uint128::from(deposit_amount),
    );

    // Fast forward time
    suite.increase_time(Duration::from_weeks(1));

    // Check how many tokens the user can withdraw with their LP tokens
    let withdraw_amount = suite
        .query_wasm_smart(contracts.lending, QueryPreviewWithdrawRequest {
            lp_tokens: Coins::one(lp_denom.clone(), lp_balance).unwrap(),
        })
        .should_succeed();

    // Check that the withdraw amount is correct. Should not have increased
    // because the interest rate is zero as there no one has borrowed from the market.
    assert_eq_or_one_off(
        withdraw_amount.amount_of(&USDC_DENOM),
        Uint128::from(deposit_amount),
    );

    // --- Property 2: Deposit and borrow interest rates are non-zero when someone has borrowed ---

    // Borrow some USDC with the margin account
    let borrow_amount = deposit_amount;
    suite
        .execute(
            margin_account,
            contracts.lending,
            &lending::ExecuteMsg::Borrow(NonEmpty::new_unchecked(
                coins! { USDC_DENOM.clone() => borrow_amount },
            )),
            Coins::new(),
        )
        .should_succeed();
    // Query the market
    let market = suite
        .query_wasm_smart(contracts.lending, QueryMarketRequest {
            denom: USDC_DENOM.clone(),
        })
        .should_succeed();

    // Compute interest rates
    let interest_rates = market
        .interest_rate_model
        .calculate_rates(market.utilization_rate(suite).unwrap());

    // Assert that the all interest rates are non-zero
    assert!(interest_rates.borrow_rate.is_positive());
    assert!(interest_rates.deposit_rate.is_positive());
    assert!(interest_rates.borrow_rate > interest_rates.deposit_rate);

    // --- Property 3: Interest accrues over time ---

    // Fast forward time
    suite.increase_time(Duration::from_weeks(1));

    // Check the margin accounts debt
    let debt = suite
        .query_wasm_smart(contracts.lending, QueryDebtRequest {
            account: margin_account.address(),
        })
        .should_succeed();
    let usdc_debt = debt.amount_of(&USDC_DENOM);
    assert!(usdc_debt > Uint128::from(borrow_amount));

    // Check that debt increased with the correct amount of interest
    let time_out_of_year = Udec128::checked_from_ratio(
        Duration::from_weeks(1).into_seconds(),
        SECONDS_PER_YEAR as u128,
    )
    .unwrap();
    let debt_interest_amount = usdc_debt - Uint128::from(borrow_amount);
    assert!(debt_interest_amount > Uint128::ZERO);
    assert_eq!(
        debt_interest_amount,
        Uint128::from(borrow_amount)
            .checked_mul_dec_ceil(interest_rates.borrow_rate * time_out_of_year)
            .unwrap()
    );

    // Check how many tokens the user can withdraw with their LP tokens
    let withdrawn_coins = suite
        .query_wasm_smart(contracts.lending, QueryPreviewWithdrawRequest {
            lp_tokens: Coins::one(lp_denom.clone(), lp_balance).unwrap(),
        })
        .should_succeed();
    let withdraw_amount = withdrawn_coins.amount_of(&USDC_DENOM);
    assert!(withdraw_amount > Uint128::from(deposit_amount));

    // Check that the withdraw amount is correct
    let deposit_interest_amount = withdraw_amount - Uint128::from(deposit_amount);
    assert_eq_or_one_off(
        deposit_interest_amount,
        Uint128::from(deposit_amount)
            .checked_mul_dec(interest_rates.deposit_rate * time_out_of_year)
            .unwrap(),
    );

    // --- Property 4: Total supply and borrow are updated with correct interest ---

    // Query the market
    let time = suite.block.timestamp;
    let market = suite
        .query_wasm_smart(contracts.lending, QueryMarketRequest {
            denom: USDC_DENOM.clone(),
        })
        .should_succeed()
        .update_indices(suite, time)
        .unwrap();
    let total_supply = market.total_supplied(suite).unwrap();
    let total_borrowed = market.total_borrowed().unwrap();

    let supply_increase = total_supply - Uint128::from(deposit_amount);
    let borrow_increase = total_borrowed - Uint128::from(borrow_amount);

    // Check that the supply and borrow increased with the correct amount of interest
    let expected_supply_increase_from_deposit = Uint128::from(deposit_amount)
        .checked_mul_dec(interest_rates.deposit_rate * time_out_of_year)
        .unwrap();
    let expected_supply_increase_from_protocol_revenue = Uint128::from(borrow_amount)
        .checked_mul_dec(
            interest_rates.borrow_rate
                * time_out_of_year
                * *market.interest_rate_model.reserve_factor,
        )
        .unwrap();
    let expected_supply_increase =
        expected_supply_increase_from_deposit + expected_supply_increase_from_protocol_revenue;
    assert_eq_or_one_off(supply_increase, expected_supply_increase);

    let expected_borrow_increase = Uint128::from(borrow_amount)
        .checked_mul_dec(interest_rates.borrow_rate * time_out_of_year)
        .unwrap();
    assert_eq_or_one_off(borrow_increase, expected_borrow_increase);

    // --- Property 5: Total supply and borrow equal the sum of deposits and debts plus interest ---

    let expected_total_supply = Uint128::from(deposit_amount) + expected_supply_increase;
    assert_eq_or_one_off(total_supply, expected_total_supply);
    assert_eq_or_one_off(
        total_borrowed,
        Uint128::from(borrow_amount) + debt_interest_amount,
    );

    // Try withdrawing deposited USDC, should fail as all the USDC is borrowed
    suite
        .execute(
            &mut accounts.user1,
            contracts.lending,
            &lending::ExecuteMsg::Withdraw {},
            Coins::one(lp_denom.clone(), lp_balance).unwrap(),
        )
        .should_fail_with_error("subtraction overflow");

    // Check the margin accounts USDC balance
    let margin_usdc_balance_before = suite
        .query_balance(&margin_account.address(), USDC_DENOM.clone())
        .should_succeed();

    // Repay the all the debt
    suite
        .execute(
            margin_account,
            contracts.lending,
            &lending::ExecuteMsg::Repay {},
            Coins::one(USDC_DENOM.clone(), usdc_debt).unwrap(),
        )
        .should_succeed();

    // Ensure the margin account's USDC balance decreased by the amount of debt repaid
    let margin_usdc_balance_after = suite
        .query_balance(&margin_account.address(), USDC_DENOM.clone())
        .should_succeed();
    assert_eq!(
        margin_usdc_balance_after,
        margin_usdc_balance_before - usdc_debt
    );

    // Query the margin account's debt. Should fail as the debt has been repaid
    suite
        .query_wasm_smart(contracts.lending, QueryDebtRequest {
            account: margin_account.address(),
        })
        .should_fail_with_error("data not found!");

    // Query the market
    let market = suite
        .query_wasm_smart(contracts.lending, QueryMarketRequest {
            denom: USDC_DENOM.clone(),
        })
        .should_succeed();
    // Ensure that total borrowed is zero
    assert_eq!(market.total_borrowed_scaled, Udec256::ZERO);

    // Check depositors USDC balance
    let depositor_usdc_balance_before = suite
        .query_balance(&accounts.user1.address(), USDC_DENOM.clone())
        .should_succeed();

    // Try withdrawing deposited USDC, should succeed
    suite
        .execute(
            &mut accounts.user1,
            contracts.lending,
            &lending::ExecuteMsg::Withdraw {},
            Coins::one(lp_denom.clone(), lp_balance).unwrap(),
        )
        .should_succeed();

    // Check depositors USDC balance. Ensure it increased by the deposited amount plus interest
    let depositor_usdc_balance_after = suite
        .query_balance(&accounts.user1.address(), USDC_DENOM.clone())
        .should_succeed();
    assert_eq!(
        depositor_usdc_balance_after - depositor_usdc_balance_before,
        Uint128::from(deposit_amount) + deposit_interest_amount
    );

    // Check the owner's USDC balance
    let owner_usdc_balance_before = suite
        .query_balance(&accounts.owner.address(), USDC_DENOM.clone())
        .should_succeed();

    // Withdraw protocol revenue
    suite
        .execute(
            &mut accounts.owner,
            contracts.lending,
            &lending::ExecuteMsg::ClaimPendingProtocolFees {},
            Coins::new(),
        )
        .should_succeed();
    let owner_lp_balance = suite
        .query_balance(&accounts.owner.address(), lp_denom.clone())
        .should_succeed();
    suite
        .execute(
            &mut accounts.owner,
            contracts.lending,
            &lending::ExecuteMsg::Withdraw {},
            Coins::one(lp_denom.clone(), owner_lp_balance).unwrap(),
        )
        .should_succeed();

    // Check the owner's USDC balance after withdrawing protocol revenue. Ensure it increased by the protocol revenue
    let owner_usdc_balance_after = suite
        .query_balance(&accounts.owner.address(), USDC_DENOM.clone())
        .should_succeed();
    assert_eq_or_one_off(
        owner_usdc_balance_after - owner_usdc_balance_before,
        expected_supply_increase_from_protocol_revenue,
    );

    // Query the market
    let market = suite
        .query_wasm_smart(contracts.lending, QueryMarketRequest {
            denom: USDC_DENOM.clone(),
        })
        .should_succeed();

    // Ensure that total supply is equal to the protocol revenueand total borrowed are zero
    assert_eq!(market.total_supplied(suite).unwrap(), Uint128::ZERO);
    assert_eq!(market.total_borrowed_scaled, Udec256::ZERO);
}
