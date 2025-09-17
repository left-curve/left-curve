use {
    dango_genesis::Contracts,
    dango_oracle::OracleQuerier,
    dango_testing::{TestAccount, TestAccounts, TestSuite, setup_test_naive},
    dango_types::{
        account::{
            self,
            margin::{CollateralPower, Liquidate, QueryHealthRequest},
            single,
        },
        account_factory::AccountParams,
        config::AppConfig,
        constants::{dango, eth, usdc},
        dex::{CreateOrderRequest, Direction},
        lending::{self, InterestRateModel, QueryDebtRequest, QueryMarketRequest},
        oracle::{self, PrecisionedPrice, PrecisionlessPrice, PriceSource, QueryPriceRequest},
    },
    grug::{
        Addr, Addressable, Binary, CheckedContractEvent, Coins, Denom, Exponentiate, Inner, IsZero,
        JsonDeExt, JsonSerExt, Message, MsgConfigure, MultiplyFraction, NextNumber, NonEmpty,
        NonZero, Number, NumberConst, PrevNumber, QuerierExt, QuerierWrapper, ResultExt,
        SearchEvent, Timestamp, Udec128, Udec128_24, Uint128, btree_map, coins,
    },
    grug_app::NaiveProposalPreparer,
    proptest::{collection::vec, prelude::*, proptest},
    pyth_types::PriceUpdate,
    std::{
        cmp::min,
        fmt::Display,
        ops::{Div, Sub},
        str::FromStr,
    },
};

/// An example Pyth VAA for an USDC price feed.
/// - id: **eaa020c61cc479712813461ce153894a96a6c00b21ed0cfc2798d1f9a9e9c94a**
/// - price: **100000966**
/// - ema_price: **99999889**
/// - expo: **-8**
/// - publish_time: **1730802926**
const USDC_VAA: &str = "UE5BVQEAAAADuAEAAAAEDQOoMTxJ5BWLUCMy94ZlQ6qBjQEzA/+ZpDKw9AGFXXSyQF2eIKCGN6cNh1f/jzNSYOf15Yk2CRvOtMc7LqzdG7NpAQSNSaXe+ZOZU4+kxAgG74ZwDUuFmTPlElG90sIMNXfFmS6WJrbTBBQNWFL2gUKpdpEp5z/wUwJo/TzB9lHDnq2vAAbYj1fi3S3mzyOvZAPbe5Qy2/L/oQdLW4FPXTVcNxjMl1m0VLYRonpvIO4/S21ovvsefil9l8R3tYNG879aE2LMAQicgal5v2vVqicVvzE2J1vhg61mEvUKKhiZhzzWo8naRgQfuvKVk3257QhmGaDaAYWxU4MJ7goFUBPbBww9gk53AQpxhRMcpv+qmFMHZCdvoWwF4I/x230bO9VOQXie1tLSf25E62lWTAdYiyrh+h/ny7GA1aDLDZYwEzT6fXUPPlg/AAuQHWuf7TcUkOuIeVisiiI5XINdK8NFu36IacZjf0okOT9dApIx4sLAReROml2hs75v4a1K8SlLB3JdQkQLMoUDAQxZDK7Rh3UBSbjTrBKe+c+5lvT6ZgP26SOqF0F26xJIqwn29C8ZzCKkDgBNzx7GbA4bwL1tNNbv6NSxyx+72AlQAQ2+4nnWuPFUrn5dJJRD5VO6CYNu42Mx4XialbPJ6Lbp3gewVGOIIiU69PyeCxX6/Q/qO99Qtc+QlDGcyjmCwQP1AA7IcDlMiDVc4wEhkfCVRxCr//C3pGZsnxZguQr0MYaSnwGQ/FzJhBsU4knRtTZgUUm3rlcwNWDAJlp5MnNcPuYpAQ91tfYjBU0lRYDoYV/00L+RgJ66vx4P4T3R3x1MuDMAalgVHg43JfcUBGytMHWSbJr/24jMWMsEPMqwBuzPvba5ABDyPKTil7cKBdhyJhTJPYNS0V9JLbS6QLPCThaTyapMMW5BQfB07Q21fXnDNZE/FSoS4JxRiKcViiwRQ4lcLE/UARHcd8PSiHsEilgDjWOH/hvpaQ+Iza+rrBithaw+nJIIdClnizW0DqO2lVx0DlERwF8C9hL3hatj888kVWzwtj6RARJoRZtdYKzWJX8KzJvlOcOBxjjiCSyfo3qLfoLHIw2rJwT7HRxqg1wXswDjq2NVjms9jz24dRIEKM0dxfEP6OckAGcp9O4AAAAAABrhAfrtrFhR4yubI7X5QRqMK6xKrj7U3XuBHdGnLqSqcQAAAAAFb/IiAUFVV1YAAAAAAAp8vHgAACcQO03kFK+kZ552XKseu11fj2cpvpUBAFUA6qAgxhzEeXEoE0Yc4VOJSpamwAsh7Qz8J5jR+anpyUoAAAAABfXkxgAAAAAAAPrQ////+AAAAABnKfTuAAAAAGcp9O4AAAAABfXgkQAAAAAAAQTcCsjx5ZH7wLv7N+2Vzze0aT71EUmuA4n/zf/zQdrI6za/FR4xTLzViierrotGyMoKwkcBs++77xpXHT1p3YXWRMQCLxEONHC/rFMy+rS7i7XohTAftvazeHYjyF6a2rZNmf+KdZS2umZMH9qPKRD3USxGDnXfQMg9mgD6HwJnHiPgaublP56r5AqPcI1tyXKMfF10MWvyxkvJbXFUuYkzW0Pi03Asu75UoUT4XeKBXfvF+EL0NmKGNrmXDYH9NpT5H6pKDeS0JDCZ";

/// Calculates the relative difference between two values.
fn relative_difference<T>(a: T, b: T) -> T
where
    T: NumberConst + Number + PartialOrd + Sub<Output = T> + Div<Output = T>,
{
    // Handle the case where both numbers are zero
    if a == T::ZERO && b == T::ZERO {
        return T::ZERO;
    }

    // Calculate absolute difference
    let abs_diff = if a > b {
        a - b
    } else {
        b - a
    };

    // Calculate the larger of the two values for relative comparison
    let larger = if a > b {
        a
    } else {
        b
    };

    // Calculate relative difference
    abs_diff / larger
}

/// Asserts that two values are approximately equal within a specified
/// relative difference.
fn assert_approx_eq<T>(a: T, b: T, max_rel_diff: &str) -> Result<(), TestCaseError>
where
    T: NumberConst + Number + PartialOrd + Sub<Output = T> + Div<Output = T> + Display,
{
    let rel_diff = Udec128::from_str(relative_difference(a, b).to_string().as_str()).unwrap();

    prop_assert!(
        rel_diff <= Udec128::from_str(max_rel_diff).unwrap(),
        "assertion failed: values are not approximately equal\n  left: {}\n right: {}\n  max_rel_diff: {}\n  actual_rel_diff: {}",
        a,
        b,
        max_rel_diff,
        rel_diff
    );

    Ok(())
}

fn feed_oracle_price(
    suite: &mut TestSuite<NaiveProposalPreparer>,
    accounts: &mut TestAccounts,
    contracts: &Contracts,
    vaa: &str,
) {
    // Push price
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::FeedPrices(PriceUpdate::Core(NonEmpty::new_unchecked(vec![
                Binary::from_str(vaa).unwrap(),
            ]))),
            Coins::default(),
        )
        .should_succeed();
}

/// Feeds the oracle contract a price for USDC
fn feed_usdc_price(
    suite: &mut TestSuite<NaiveProposalPreparer>,
    accounts: &mut TestAccounts,
    contracts: &Contracts,
) {
    feed_oracle_price(suite, accounts, contracts, USDC_VAA);
}

/// Sets the collateral power for a given denom
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

/// Helper function to mint several coins
fn mint_coins(
    suite: &mut TestSuite<NaiveProposalPreparer>,
    accounts: &mut TestAccounts,
    contracts: &Contracts,
    address: Addr,
    coins: Coins,
) {
    suite
        .execute(
            &mut accounts.owner,
            contracts.bank,
            &dango_types::bank::ExecuteMsg::Mint { to: address, coins },
            Coins::new(),
        )
        .should_succeed();
}

/// Helper function to register a fixed price for a collateral
fn register_fixed_price(
    suite: &mut TestSuite<NaiveProposalPreparer>,
    accounts: &mut TestAccounts,
    contracts: &Contracts,
    denom: Denom,
    humanized_price: Udec128,
    precision: u8,
) {
    // Register price source
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &dango_types::oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                denom => dango_types::oracle::PriceSource::Fixed {
                    humanized_price,
                    precision,
                    timestamp: Timestamp::from_seconds(0),
                }
            }),
            Coins::default(),
        )
        .should_succeed();
}

#[test]
fn margin_account_creation() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    // Create a margin account.
    let username = accounts.user1.username.clone();

    accounts
        .user1
        .register_new_account(
            &mut suite,
            contracts.account_factory,
            AccountParams::Margin(single::Params::new(username)),
            Coins::new(),
        )
        .should_succeed();
}

/// Some standard setup that needs to be done to get margin accounts working.
/// Does the following:
/// - feeds the oracle with a price for USDC (~$1) and ETH (~$71K)
/// - creates a margin account
/// - deposits some USDC into the lending pool
/// - deposits some ETH into the lending pool
/// - whitelists USDC as collateral at 100% power
/// - whitelists ETH as collateral at 80% power
fn setup_margin_test_env(
    suite: &mut TestSuite<NaiveProposalPreparer>,
    accounts: &mut TestAccounts,
    contracts: &Contracts,
) -> TestAccount {
    feed_usdc_price(suite, accounts, contracts);
    register_fixed_price(
        suite,
        accounts,
        contracts,
        eth::DENOM.clone(),
        Udec128::from_str("71319.50295749").unwrap(),
        18,
    );

    // Create a margin account.
    let username = accounts.user1.username.clone();
    let margin_account = accounts
        .user1
        .register_new_account(
            suite,
            contracts.account_factory,
            AccountParams::Margin(single::Params::new(username)),
            Coins::new(),
        )
        .should_succeed();

    // Deposit some USDC to the lending pool
    suite
        .execute(
            &mut accounts.user1,
            contracts.lending,
            &lending::ExecuteMsg::Deposit {},
            Coins::one(usdc::DENOM.clone(), 10_000_000_000).unwrap(),
        )
        .should_succeed();

    // Whitelist USDC as collateral at 100% power
    set_collateral_power(
        suite,
        accounts,
        usdc::DENOM.clone(),
        CollateralPower::new(Udec128::new_percent(100)).unwrap(),
    );

    // Deposit some ETH to the lending pool
    suite
        .execute(
            &mut accounts.user1,
            contracts.lending,
            &lending::ExecuteMsg::Deposit {},
            Coins::one(eth::DENOM.clone(), 10 * 10_u128.pow(18)).unwrap(), // 10 ETH
        )
        .should_succeed();

    // Whitelist ETH as collateral at 80% power
    set_collateral_power(
        suite,
        accounts,
        eth::DENOM.clone(),
        CollateralPower::new(Udec128::new_percent(80)).unwrap(),
    );

    margin_account
}

#[test]
fn cant_liquidate_when_overcollateralised() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());
    let mut margin_account = setup_margin_test_env(&mut suite, &mut accounts, &contracts);

    // Borrow with the margin account
    suite
        .execute(
            &mut margin_account,
            contracts.lending,
            &lending::ExecuteMsg::Borrow(NonEmpty::new_unchecked(
                coins! { usdc::DENOM.clone() => 100_000_000 },
            )),
            Coins::new(),
        )
        .should_succeed();

    // Try to liquidate the margin account, should fail as it's not undercollateralized
    suite
        .execute(
            &mut accounts.user1,
            margin_account.address(),
            &account::margin::ExecuteMsg::Liquidate {
                collateral: usdc::DENOM.clone(),
            },
            Coins::new(),
        )
        .should_fail_with_error("account is not undercollateralized");
}

#[test]
fn liquidation_works() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());
    let mut margin_account = setup_margin_test_env(&mut suite, &mut accounts, &contracts);

    // Borrow with the margin account
    suite
        .execute(
            &mut margin_account,
            contracts.lending,
            &lending::ExecuteMsg::Borrow(NonEmpty::new_unchecked(
                coins! { usdc::DENOM.clone() => 100_000_000 },
            )),
            Coins::new(),
        )
        .should_succeed();

    // Confirm the margin account has the borrowed coins
    suite
        .query_balance(&margin_account.address(), usdc::DENOM.clone())
        .should_succeed_and_equal(Uint128::new(100_000_000));

    // Update USDC collateral power to 90% to make the account undercollateralised
    set_collateral_power(
        &mut suite,
        &mut accounts,
        usdc::DENOM.clone(),
        CollateralPower::new(Udec128::new_percent(90)).unwrap(),
    );

    // Check margin account's debts before
    let debts_before = suite
        .query_wasm_smart(contracts.lending, QueryDebtRequest {
            account: margin_account.address(),
        })
        .unwrap();

    // Check liquidator account's USDC balance before
    let balance_before = suite
        .query_balance(&accounts.user1.address(), usdc::DENOM.clone())
        .unwrap();

    // Try to partially liquidate the margin account, should succeed
    suite
        .execute(
            &mut accounts.user1,
            margin_account.address(),
            &account::margin::ExecuteMsg::Liquidate {
                collateral: usdc::DENOM.clone(),
            },
            Coins::one(usdc::DENOM.clone(), 50_000_000).unwrap(),
        )
        .should_succeed();

    // Check liquidator account's USDC balance after
    let balance_after = suite
        .query_balance(&accounts.user1.address(), usdc::DENOM.clone())
        .unwrap();

    // Ensure balance increased (should receive collateral plus bonus worth more than the repaid debt)
    assert!(balance_after > balance_before);

    // Account's debts should have decreased by the amount of the liquidation
    let debts_after = suite
        .query_wasm_smart(contracts.lending, QueryDebtRequest {
            account: margin_account.address(),
        })
        .unwrap();

    // Since this is a partial liquidation, ensure the debt has decreased exactly with the sent amount
    assert_eq!(
        debts_before.amount_of(&usdc::DENOM) - debts_after.amount_of(&usdc::DENOM),
        Uint128::new(50_000_000)
    );

    // Try to liquidate the rest of the account's collateral, should succeed
    suite
        .execute(
            &mut accounts.user1,
            margin_account.address(),
            &account::margin::ExecuteMsg::Liquidate {
                collateral: usdc::DENOM.clone(),
            },
            Coins::one(usdc::DENOM.clone(), 100_000_000).unwrap(),
        )
        .should_succeed();

    // Check liquidator account's USDC balance after
    let balance_before = balance_after;
    let balance_after = suite
        .query_balance(&accounts.user1.address(), usdc::DENOM.clone())
        .unwrap();

    // Ensure balance increased (should receive collateral plus bonus worth more than the repaid debt)
    assert!(balance_after > balance_before);

    // Account's debts should have decreased
    let debts_before = debts_after;
    let debts_after = suite
        .query_wasm_smart(contracts.lending, QueryDebtRequest {
            account: margin_account.address(),
        })
        .unwrap();

    // This liquidation incurred bad debt, so we just check that the debts have decreased
    assert!(debts_before.amount_of(&usdc::DENOM) > debts_after.amount_of(&usdc::DENOM));

    // Ensure the account has no collateral left
    suite
        .query_balance(&margin_account.address(), usdc::DENOM.clone())
        .should_succeed_and_equal(Uint128::ZERO);
}

#[test]
fn liquidation_works_with_multiple_debt_denoms() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());
    let mut margin_account = setup_margin_test_env(&mut suite, &mut accounts, &contracts);

    // Borrow some USDC with the margin account
    suite
        .execute(
            &mut margin_account,
            contracts.lending,
            &lending::ExecuteMsg::Borrow(NonEmpty::new_unchecked(
                coins! { usdc::DENOM.clone() => 1_000_000_000 }, // 1K USDC
            )),
            Coins::new(),
        )
        .should_succeed();

    // Send some more USDC to the margin account as collateral
    suite
        .transfer(
            &mut accounts.user1,
            margin_account.address(),
            Coins::one(usdc::DENOM.clone(), 15_000_000_000).unwrap(), // 15k USDC
        )
        .should_succeed();

    // Borrow some ETH
    suite
        .execute(
            &mut margin_account,
            contracts.lending,
            &lending::ExecuteMsg::Borrow(NonEmpty::new_unchecked(
                coins! { eth::DENOM.clone() => 10u128.pow(18) }, // 1 ETH
            )),
            Coins::new(),
        )
        .should_succeed();

    // Query account's health
    let health = suite
        .query_wasm_smart(margin_account.address(), QueryHealthRequest {
            skip_if_no_debt: false,
        })
        .unwrap()
        .unwrap();
    assert!(health.utilization_rate < Udec128::ONE);

    // Update the oracle price of ETH to go from $71k to $96k, making the account undercollateralised
    register_fixed_price(
        &mut suite,
        &mut accounts,
        &contracts,
        eth::DENOM.clone(),
        Udec128::from_str("96319.50295749").unwrap(),
        18,
    );

    // Query account's health
    let health = suite
        .query_wasm_smart(margin_account.address(), QueryHealthRequest {
            skip_if_no_debt: false,
        })
        .unwrap()
        .unwrap();
    assert!(health.utilization_rate > Udec128::ONE);

    let debts_before = health.debts;
    // Add one microunit as debt may have increased by the time we liquidate due to interest
    let usdc_repay_amount = debts_before.amount_of(&usdc::DENOM).into_inner() + 1;

    // Check liquidator account's USDC balance before
    let usdc_balance_before = suite
        .query_balance(&accounts.user1.address(), usdc::DENOM.clone())
        .unwrap();

    // Try to partially liquidate the margin account, fully paying off USDC debt,
    // and some of the ETH debt
    suite
        .execute(
            &mut accounts.user1,
            margin_account.address(),
            &account::margin::ExecuteMsg::Liquidate {
                collateral: usdc::DENOM.clone(),
            },
            coins! {
                usdc::DENOM.clone() => usdc_repay_amount,
                eth::DENOM.clone() => 10_u128.pow(15), // 0.001 ETH
            },
        )
        .should_succeed();

    // Check liquidator account's USDC balance after
    let usdc_balance_after = suite
        .query_balance(&accounts.user1.address(), usdc::DENOM.clone())
        .unwrap();

    // Ensure liquidators USDC balance increased
    assert!(usdc_balance_after > usdc_balance_before);

    // Account's debts should have decreased by the amount of the liquidation
    let debts_after = suite
        .query_wasm_smart(contracts.lending, QueryDebtRequest {
            account: margin_account.address(),
        })
        .unwrap();

    // Ensure the USDC debt was fully paid off
    assert!(debts_after.amount_of(&usdc::DENOM).is_zero());

    // Try to liquidate the rest of the account's ETH collateral, but send USDC
    // to cover the debt. Should fail since the account no longer has USDC debt.
    suite
        .execute(
            &mut accounts.user1,
            margin_account.address(),
            &account::margin::ExecuteMsg::Liquidate {
                collateral: eth::DENOM.clone(),
            },
            Coins::one(usdc::DENOM.clone(), 100_000_000).unwrap(),
        )
        .should_fail_with_error("no debt was repaid");

    // Check liquidator account's ETH balance before
    let eth_balance_before = suite
        .query_balance(&accounts.user1.address(), eth::DENOM.clone())
        .unwrap();

    // Try to liquidate the rest of the account's collateral, should succeed
    suite
        .execute(
            &mut accounts.user1,
            margin_account.address(),
            &account::margin::ExecuteMsg::Liquidate {
                collateral: eth::DENOM.clone(),
            },
            Coins::one(eth::DENOM.clone(), 10u128.pow(18)).unwrap(),
        )
        .should_succeed();

    // Check liquidator account's ETH balance after
    let eth_balance_after = suite
        .query_balance(&accounts.user1.address(), eth::DENOM.clone())
        .unwrap();

    // Ensure balance increased (should receive collateral plus bonus worth more than the repaid debt)
    assert!(eth_balance_after > eth_balance_before);

    // Query account's health
    let health = suite
        .query_wasm_smart(margin_account.address(), QueryHealthRequest {
            skip_if_no_debt: false,
        })
        .unwrap()
        .unwrap();
    let app_config: AppConfig = suite.query_app_config().unwrap();

    // Ensure the new utilization rate is equal to the target utilization rate
    assert_approx_eq(
        health.utilization_rate,
        *app_config.target_utilization_rate,
        "0.0001",
    )
    .unwrap();

    // Check that the debt after is correct (using manual calculation via equations)
    assert_approx_eq(
        health.total_debt_value,
        Udec128::from_str("41609.67023").unwrap(),
        "0.006",
    )
    .unwrap();
    let debts_after = suite
        .query_wasm_smart(contracts.lending, QueryDebtRequest {
            account: margin_account.address(),
        })
        .unwrap();
    assert_eq!(
        debts_after.amount_of(&eth::DENOM),
        Uint128::new(434550749110464888)
    );

    // Check that the collateral value after is correct (using manual calculation via equations)
    assert_approx_eq(
        health.total_adjusted_collateral_value,
        Udec128::from_str("46232.96693").unwrap(),
        "0.006",
    )
    .unwrap();
}

#[test]
fn tokens_deposited_into_lending_pool_are_counted_as_collateral() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());
    let mut margin_account = setup_margin_test_env(&mut suite, &mut accounts, &contracts);

    // Send some USDC to the margin account as collateral (needed to cover interest from borrowing)
    suite
        .transfer(
            &mut accounts.user1,
            margin_account.address(),
            Coins::one(usdc::DENOM.clone(), 100_000_000).unwrap(),
        )
        .should_succeed();

    // Borrow some USDC with the margin account
    suite
        .execute(
            &mut margin_account,
            contracts.lending,
            &lending::ExecuteMsg::Borrow(NonEmpty::new_unchecked(
                coins! { usdc::DENOM.clone() => 1_000_000_000 }, // 1K USDC
            )),
            Coins::new(),
        )
        .should_succeed();

    // Try to deposit 100 USDC into the lending pool, should fail since it's not listed as collateral yet
    suite
        .execute(
            &mut margin_account,
            contracts.lending,
            &lending::ExecuteMsg::Deposit {},
            Coins::one(usdc::DENOM.clone(), 1_000_000_000).unwrap(),
        )
        .should_fail_with_error("this action would make account undercollateralized!");

    // Query market for USDC
    let market = suite
        .query_wasm_smart(contracts.lending, QueryMarketRequest {
            denom: usdc::DENOM.clone(),
        })
        .unwrap();

    // List the LP token as collateral at 100% power
    set_collateral_power(
        &mut suite,
        &mut accounts,
        market.supply_lp_denom.clone(),
        CollateralPower::new(Udec128::new_percent(100)).unwrap(),
    );

    // Register price source for LP token
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                market.supply_lp_denom.clone() => PriceSource::LendingLiquidity,
            }),
            Coins::new(),
        )
        .should_succeed();

    // Try to deposit 100 USDC into the lending pool, should succeed
    suite
        .execute(
            &mut margin_account,
            contracts.lending,
            &lending::ExecuteMsg::Deposit {},
            Coins::one(usdc::DENOM.clone(), 1_000_000_000).unwrap(),
        )
        .should_succeed();

    // Query LP token balance
    let lp_balance = suite
        .query_balance(&margin_account.address(), market.supply_lp_denom.clone())
        .unwrap();
    assert!(lp_balance.is_non_zero());
}

#[test]
fn limit_orders_are_counted_as_collateral_and_can_be_liquidated() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());
    let mut margin_account = setup_margin_test_env(&mut suite, &mut accounts, &contracts);

    // Send some USDC to the margin account as collateral
    suite
        .transfer(
            &mut accounts.user1,
            margin_account.address(),
            Coins::one(usdc::DENOM.clone(), 100_000_000).unwrap(),
        )
        .should_succeed();

    // Register fixed price source for dango
    register_fixed_price(
        &mut suite,
        &mut accounts,
        &contracts,
        dango::DENOM.clone(),
        Udec128::ONE,
        6,
    );

    // Set collateral power for DANGO at 100%
    set_collateral_power(
        &mut suite,
        &mut accounts,
        dango::DENOM.clone(),
        CollateralPower::new(Udec128::new_percent(100)).unwrap(),
    );

    // Create a limit order
    suite
        .execute(
            &mut margin_account,
            contracts.dex,
            &dango_types::dex::ExecuteMsg::BatchUpdateOrders {
                creates: vec![CreateOrderRequest::new_limit(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    NonZero::new_unchecked(Udec128_24::ONE),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                )],
                cancels: None,
            },
            Coins::one(usdc::DENOM.clone(), 100_000_000).unwrap(),
        )
        .should_succeed();

    // Query account's health and ensure the limit order is counted as collateral
    let health = suite
        .query_wasm_smart(margin_account.address(), QueryHealthRequest {
            skip_if_no_debt: false,
        })
        .unwrap()
        .unwrap();
    assert_eq!(
        health.total_adjusted_collateral_value,
        Udec128::from_str("100").unwrap(),
    );
    assert_eq!(
        health.limit_order_collaterals,
        Coins::one(usdc::DENOM.clone(), 100_000_000).unwrap(),
    );
    assert_eq!(
        health.limit_order_outputs,
        Coins::one(dango::DENOM.clone(), 100_000_000).unwrap(),
    );
    assert_eq!(health.collaterals, Coins::new());

    // Borrow some ETH with the margin account
    suite
        .execute(
            &mut margin_account,
            contracts.lending,
            &lending::ExecuteMsg::Borrow(NonEmpty::new_unchecked(
                coins! { eth::DENOM.clone() => 6 * 10u128.pow(15) }, // 0.006 ETH = 426 USD
            )),
            Coins::new(),
        )
        .should_succeed();

    // Feed the oracle price of ETH to $96k to make the account undercollateralised
    register_fixed_price(
        &mut suite,
        &mut accounts,
        &contracts,
        eth::DENOM.clone(),
        Udec128::from_str("96319.50295749").unwrap(),
        18,
    );

    // Query account's health to ensure it is undercollateralised
    let health = suite
        .query_wasm_smart(margin_account.address(), QueryHealthRequest {
            skip_if_no_debt: false,
        })
        .unwrap()
        .unwrap();
    assert!(health.utilization_rate > Udec128::ONE);

    // Check liquidator account's USDC and ETH balance before
    let usdc_balance_before = suite
        .query_balance(&accounts.user1.address(), usdc::DENOM.clone())
        .unwrap();
    let eth_balance_before = suite
        .query_balance(&accounts.user1.address(), eth::DENOM.clone())
        .unwrap();

    // Liquidate the margin account
    suite
        .execute(
            &mut accounts.user1,
            margin_account.address(),
            &account::margin::ExecuteMsg::Liquidate {
                collateral: usdc::DENOM.clone(),
            },
            Coins::one(eth::DENOM.clone(), 6 * 10u128.pow(15)).unwrap(),
        )
        .should_succeed();

    // Check liquidator account's USDC and ETH balance after
    let usdc_balance_after = suite
        .query_balance(&accounts.user1.address(), usdc::DENOM.clone())
        .unwrap();
    let eth_balance_after = suite
        .query_balance(&accounts.user1.address(), eth::DENOM.clone())
        .unwrap();

    // Ensure liquidator received the liquidated collateral and bonus
    let mut oracle_querier =
        OracleQuerier::new_remote(contracts.oracle.address(), QuerierWrapper::new(&suite));
    let eth_price = oracle_querier.query_price(&eth::DENOM, None).unwrap();
    let liquidator_usdc_increase = usdc_balance_after.checked_sub(usdc_balance_before).unwrap();
    let liquidator_eth_decrease = eth_balance_before.checked_sub(eth_balance_after).unwrap();
    assert!(liquidator_usdc_increase > Uint128::new(95_000_000));
    assert_approx_eq(
        liquidator_eth_decrease,
        eth_price.unit_amount_from_value(Udec128::new(100)).unwrap(),
        "0.0001",
    )
    .unwrap();

    // Query account's health to ensure it has been liquidated
    let health = suite
        .query_wasm_smart(margin_account.address(), QueryHealthRequest {
            skip_if_no_debt: false,
        })
        .unwrap()
        .unwrap();
    assert!(health.limit_order_collaterals.is_empty());
    assert!(health.limit_order_outputs.is_empty());
    assert!(health.collaterals.amount_of(&usdc::DENOM) < Uint128::new(100)); // some dust left
    assert_eq!(
        health.debts.amount_of(&eth::DENOM),
        Uint128::new(6 * 10u128.pow(15)) - liquidator_eth_decrease,
    );
}

#[derive(Debug, Clone)]
struct TestDenom {
    denom: Denom,
    initial_price: PrecisionedPrice,
}

#[derive(Debug, Clone)]
struct Collateral {
    denom: TestDenom,
    amount: Uint128,
    collateral_power: CollateralPower,
}

#[derive(Debug, Clone)]
struct Debt {
    denom: TestDenom,
    amount: Uint128,
}

/// Proptest strategy for generating a single Denom
fn denom(index: usize) -> impl Strategy<Value = Denom> {
    Just(Denom::from_str(&format!("denom{index}")).unwrap())
}

/// Proptest strategy for generating a single TestDenom
fn test_denom(index: usize) -> impl Strategy<Value = TestDenom> {
    (
        denom(index),
        // Precision between 6-18 decimals
        (6u8..=18u8),
        // Initial price between 0.01 and 10M USD
        (1u128..1_000_000_000u128).prop_map(Udec128::new_percent),
    )
        .prop_map(|(denom, precision, price)| TestDenom {
            denom,
            initial_price: PrecisionlessPrice::new(price, Timestamp::from_seconds(0))
                .with_precision(precision),
        })
}

/// Proptest strategy for generating a set of test denoms
fn test_denoms(min_size: usize, max_size: usize) -> impl Strategy<Value = Vec<TestDenom>> {
    // Generate size first
    (min_size..=max_size).prop_flat_map(move |size| {
        // Generate vec of indices from 1 to size and map to test denoms
        (1..=size)
            .collect::<Vec<_>>()
            .into_iter()
            .map(test_denom)
            .collect::<Vec<_>>()
    })
}

/// Proptest strategy for generating a collateral
fn collateral(denom: TestDenom) -> impl Strategy<Value = Collateral> {
    (
        // Value between $20 and $10M
        (20u128..10_000_000u128).prop_map(Udec128::new),
        // Collateral power between 30% and 95%
        (30u128..95u128).prop_map(|x| CollateralPower::new(Udec128::new_percent(x)).unwrap()),
    )
        .prop_map(move |(value, collateral_power)| {
            let amount = denom.initial_price.unit_amount_from_value(value).unwrap();
            Collateral {
                denom: denom.clone(),
                amount,
                collateral_power,
            }
        })
}

/// Represents a test scenario for a liquidation.
/// The scenario contains the initial collaterals and debts for a margin account,
/// along with a selected debt denom whose price will be changed to trigger liquidation.
#[derive(Debug, Clone)]
struct LiquidationScenario {
    /// The denoms used for both collaterals and debts
    test_denoms: Vec<TestDenom>,
    /// The collaterals of the margin account before liquidation
    collaterals: Vec<Collateral>,
    /// The debts of the margin account before liquidation
    debts: Vec<Debt>,
    /// The denom whose price will be changed to trigger liquidation
    changed_denom: TestDenom,
    /// The new price of the changed denom
    new_price: Udec128,
}

/// Proptest strategy for generating a liquidation scenario
fn liquidation_scenario() -> impl Strategy<Value = LiquidationScenario> {
    (
        // Generate a single set of denoms that will be used for both collaterals and debts
        test_denoms(2, 7),
        // Generate initial utilization rate from 80% to 99%
        (80u128..=99u128).prop_map(Udec128::new_percent),
        // Generate utilization rate after price change from 101% to 150%
        (101u128..=150u128).prop_map(Udec128::new_percent),
    )
        .prop_flat_map(
            |(all_denoms, initial_utilization_rate, utilization_rate_after_price_change)| {
                // From the set of `all_denoms`, pick 1-3 randomly for debts and select one as the denom whose price will be changed
                let debt_denoms =
                    prop::sample::subsequence(all_denoms.clone(), 1..min(all_denoms.len(), 3))
                        .prop_flat_map(|denoms| {
                            let changed_denom = prop::sample::select(denoms.clone());
                            (Just(denoms), changed_denom)
                        });

                (
                    Just(all_denoms),
                    debt_denoms,
                    Just(initial_utilization_rate),
                    Just(utilization_rate_after_price_change),
                )
            },
        )
        .prop_flat_map(
            |(
                all_denoms,
                (debt_denoms, changed_denom),
                initial_utilization_rate,
                utilization_rate_after_price_change,
            )| {
                // Generate how many percent of the total debt value each debt denom should be
                let debt_percentages =
                    vec(1u128..100, debt_denoms.len()).prop_map(move |weights| {
                        let sum: u128 = weights.iter().sum();
                        weights
                            .into_iter()
                            .map(|w| Udec128::checked_from_ratio(w, sum).unwrap())
                            .collect::<Vec<_>>()
                    });

                // Select 1-4 collateral denoms from the set of `all_denoms` excluding the changed debt denom.
                let choices: Vec<TestDenom> = all_denoms
                    .clone()
                    .into_iter()
                    .filter(|x| x.denom != changed_denom.denom)
                    .collect();
                let collaterals =
                    prop::sample::subsequence(choices.clone(), 1..=min(choices.len(), 4))
                        .prop_flat_map(|denoms| {
                            denoms.into_iter().map(collateral).collect::<Vec<_>>()
                        });

                (
                    Just(all_denoms),
                    collaterals,
                    Just(debt_denoms),
                    Just(changed_denom),
                    debt_percentages,
                    Just(initial_utilization_rate),
                    Just(utilization_rate_after_price_change),
                )
            },
        )
        .prop_flat_map(
            |(
                all_denoms,
                collaterals,
                debt_denoms,
                changed_denom,
                debt_percentages,
                initial_utilization_rate,
                utilization_rate_after_price_change,
            )| {
                // Generate debts such that the initial utilization rate is met
                // Since the debts can also act as collaterals we must solve this equation:
                // where: u = utilization rate, d = debt value, c = adjusted collateral value, p = average collateral power of debt denoms
                // u = d / c => u = d / (c + d * p)
                // And then solve for d to get:
                // d = (u * c) / (1 - u * p)
                let total_adjusted_collateral_value = collaterals
                    .iter()
                    .map(|c| {
                        c.denom
                            .initial_price
                            .value_of_unit_amount(c.amount)
                            .unwrap()
                            * *c.collateral_power
                    })
                    .fold(Udec128::ZERO, |acc, x| acc + x);

                let average_debt_collateral_power = debt_denoms
                    .iter()
                    .zip(debt_percentages.clone())
                    .filter(|(d, _)| collaterals.iter().any(|c| c.denom.denom == d.denom))
                    .map(|(denom, percentage)| {
                        *collaterals
                            .iter()
                            .find(|c| c.denom.denom == denom.denom)
                            .unwrap()
                            .collateral_power
                            * percentage
                    })
                    .fold(Udec128::ZERO, |acc, x| acc + x);

                let total_debt_value = (initial_utilization_rate * total_adjusted_collateral_value)
                    / (Udec128::ONE - initial_utilization_rate * average_debt_collateral_power);

                let debts: Vec<Debt> = debt_denoms
                    .iter()
                    .zip(debt_percentages.clone())
                    .map(|(denom, percentage)| {
                        let value = total_debt_value * percentage;
                        Debt {
                            denom: denom.clone(),
                            amount: denom.initial_price.unit_amount_from_value(value).unwrap(),
                        }
                    })
                    .collect();

                // Next, generate the price change such that the utilization rate after the price change is met
                let changed_debt = debts
                    .iter()
                    .find(|d| d.denom.denom == changed_denom.denom)
                    .unwrap();

                // Get the value of all the other debts
                let other_debt_values = total_debt_value
                    - changed_debt
                        .denom
                        .initial_price
                        .value_of_unit_amount(changed_debt.amount)
                        .unwrap();

                // The ajusted value of all debt denoms not including the changed denom
                let other_debt_adjusted_value = debts
                    .iter()
                    .filter(|d| collaterals.iter().any(|c| c.denom.denom == d.denom.denom))
                    .filter(|d| d.denom.denom != changed_debt.denom.denom)
                    .map(|debt| {
                        debt.denom
                            .initial_price
                            .value_of_unit_amount(debt.amount)
                            .unwrap()
                            * *collaterals
                                .iter()
                                .find(|c| c.denom.denom == debt.denom.denom)
                                .unwrap()
                                .collateral_power
                    })
                    .fold(Udec128::ZERO, |acc, x| acc + x);

                let price_after: Udec128 = ((utilization_rate_after_price_change
                    * (total_adjusted_collateral_value + other_debt_adjusted_value)
                    - other_debt_values)
                    .into_next()
                    / changed_debt.amount.into_next().checked_into_dec().unwrap())
                .checked_into_prev()
                .unwrap();

                let price_after_humanized = price_after
                    * Udec128::new(10)
                        .checked_pow(changed_debt.denom.initial_price.precision() as u32)
                        .unwrap();

                (
                    Just(all_denoms),
                    Just(collaterals),
                    Just(debts),
                    Just(price_after_humanized),
                    Just(changed_denom),
                )
            },
        )
        .prop_map(
            |(all_denoms, collaterals, debts, new_price, changed_denom)| LiquidationScenario {
                test_denoms: all_denoms,
                collaterals,
                debts,
                new_price,
                changed_denom,
            },
        )
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 128,
        max_local_rejects: 0,
        max_global_rejects: 0,
        max_shrink_iters: 128,
        verbose: 1,
        ..ProptestConfig::default()
    })]

    /// Uses proptest to generate a random liquidation scenarios and tests them
    #[test]
    fn test_liquidation_scenario(scenario in liquidation_scenario()) {
        let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

        // Create margin account that will borrow and be liquidated
        let username = accounts.user1.username.clone();
        let mut margin_account = accounts
            .user1
            .register_new_account(
                &mut suite,
                contracts.account_factory,
                AccountParams::Margin(single::Params::new(username.clone())),
                Coins::new(),
            )
            .should_succeed();

        // Create spot account as liquidator
        let mut liquidator = accounts
            .user1
            .register_new_account(
                &mut suite,
                contracts.account_factory,
                AccountParams::Spot(single::Params::new(username.clone())),
                Coins::new(),
            )
            .should_succeed();

        // Setup prices for all denoms
        for denom in &scenario.test_denoms {
            register_fixed_price(
                &mut suite,
                &mut accounts,
                &contracts,
                denom.denom.clone(),
                denom.initial_price.humanized_price,
                denom.initial_price.precision(),
            );
        }

        for collateral in &scenario.collaterals {
            // Set collateral power for each collateral
            set_collateral_power(
                &mut suite,
                &mut accounts,
                collateral.denom.denom.clone(),
                collateral.collateral_power,
            );

            // Mint collateral to margin account
            mint_coins(
                &mut suite,
                &mut accounts,
                &contracts,
                margin_account.address(),
                Coins::one(collateral.denom.denom.clone(), collateral.amount).unwrap(),
            );
        }

        // Mint debt denoms, provide to lending market, and borrow against it
        for debt in &scenario.debts {
            // Mint debt denom to the liquidator account (for repaying debt). Mint some extra to cover interest.
            mint_coins(
                &mut suite,
                &mut accounts,
                &contracts,
                liquidator.address(),
                Coins::one(debt.denom.denom.clone(), debt.amount.checked_mul_dec(Udec128::new_percent(110)).unwrap()).unwrap(),
            );

            // Mint debt denom to the user account
            let user = accounts.user1.address();
            mint_coins(
                &mut suite,
                &mut accounts,
                &contracts,
                user,
                Coins::one(debt.denom.denom.clone(), debt.amount).unwrap(),
            );

            // Create borrow/lend market for each denom
            suite
                .execute(
                    &mut accounts.owner,
                    contracts.lending,
                    &lending::ExecuteMsg::UpdateMarkets(btree_map! {
                        debt.denom.denom.clone() => InterestRateModel::mock(),
                    }),
                    Coins::new(),
                )
                .should_succeed();

            // Provide to lending market from the user account
            suite
                .execute(
                    &mut accounts.user1,
                    contracts.lending,
                    &lending::ExecuteMsg::Deposit {},
                    Coins::one(debt.denom.denom.clone(), debt.amount).unwrap(),
                )
                .should_succeed();

            // Borrow the coins from the lending market with the margin account
            suite
                .execute(
                    &mut margin_account,
                    contracts.lending,
                    &lending::ExecuteMsg::Borrow(NonEmpty::new_unchecked(
                        coins! { debt.denom.denom.clone() => debt.amount },
                    )),
                    Coins::new(),
                )
                .should_succeed();
        }

        // Change price of chosen debt denom to make margin account undercollateralized
        register_fixed_price(
            &mut suite,
            &mut accounts,
            &contracts,
            scenario.changed_denom.denom.clone(),
            scenario.new_price,
            scenario.changed_denom.initial_price.precision(),
        );

        // Check margin accounts health
        let margin_account_health = suite
            .query_wasm_smart(margin_account.address(), QueryHealthRequest {
                skip_if_no_debt: false,
            })
            .unwrap()
            .unwrap();

        // Get liquidators total account value before liquidation
        let liquidator_balances_before = suite.query_balances(&liquidator).unwrap();
        let liquidator_worth_before = liquidator_balances_before
            .clone()
            .into_iter().map(|coin| {
                let price = suite
                    .query_wasm_smart(contracts.oracle, QueryPriceRequest {
                        denom: coin.denom,
                    })
                    .unwrap();
                price.value_of_unit_amount::<18>(coin.amount).unwrap()
            })
            .reduce(|a, b| a + b)
            .unwrap();

        // Attempt liquidation
        let res = suite
            .execute(
                &mut liquidator,
                margin_account.address(),
                &account::margin::ExecuteMsg::Liquidate {
                    collateral: scenario.collaterals[0].denom.denom.clone(),
                },
                margin_account_health.debts.clone(),
            )
            .should_succeed();

        // Get liquidators total account value after liquidation
        let liquidator_balances_after = suite.query_balances(&liquidator).unwrap();
        let liquidator_worth_after = liquidator_balances_after
            .into_iter()
            .map(|coin| {
                let price = suite
                    .query_wasm_smart(contracts.oracle, QueryPriceRequest {
                        denom: coin.denom,
                    })
                    .unwrap();
                price.value_of_unit_amount(coin.amount).unwrap()
            })
            .reduce(|a, b| a + b)
            .unwrap();

        // Property: Liquidation should always result in a profit for the liquidator
        prop_assert!(
            liquidator_worth_after > liquidator_worth_before,
            "Liquidation should result in profit for liquidator"
        );

        // Check liquidation bonus
        let config = suite.query_app_config::<AppConfig>().unwrap();
        let liquidation_event = res.events
            .search_event::<CheckedContractEvent>()
            .with_predicate(|e| e.ty == "liquidate")
            .take()
            .one()
            .event
            .data
            .deserialize_json::<Liquidate>()
            .unwrap();
        let repaid_debt_value = liquidation_event.repaid_debt_value;
        let claimed_collateral_amount = liquidation_event.claimed_collateral_amount;
        let claimed_collateral_value = suite
            .query_wasm_smart(contracts.oracle, QueryPriceRequest {
                denom: scenario.collaterals[0].denom.denom.clone(),
            })
            .unwrap()
            .value_of_unit_amount(claimed_collateral_amount)
            .unwrap();
        let liquidation_bonus = (claimed_collateral_value - repaid_debt_value) / repaid_debt_value;
        let liquidation_bonus_from_event: Udec128 = liquidation_event.liquidation_bonus;

        // Property: Liquidation bonus is within the bounds
        prop_assert!(
            liquidation_bonus_from_event >= *config.min_liquidation_bonus
                && liquidation_bonus_from_event <= *config.max_liquidation_bonus,
            "Liquidation bonus should be within the bounds"
        );

        // Property: Actual liquidation bonus should be very close to the configured value
        // We only run this check if the collateral amount is more than 10000 microunits,
        // since otherwise the rounding will cause the actual bonus to be much larger
        // than the configured value.
        if scenario.collaterals[0].amount > Uint128::new(10000) {
            assert_approx_eq(liquidation_bonus, liquidation_bonus_from_event, "0.02")?;
        } else {
            // If collateral amount is very small, rounding will occur so we just
            // check that the liquidation bonus is larger than the configured value
            // (so that liquidators don't lose out)
            prop_assert!(liquidation_bonus >= liquidation_bonus_from_event);
        }
    }
}
