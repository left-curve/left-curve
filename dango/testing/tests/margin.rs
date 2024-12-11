use {
    dango_genesis::{Codes, Contracts},
    dango_testing::{setup_test_naive, TestAccount, TestAccounts, TestSuite},
    dango_types::{
        account::{
            self,
            margin::{CollateralPower, QueryHealthRequest},
            single,
        },
        account_factory::AccountParams,
        config::AppConfig,
        lending::{self, MarketUpdates, QueryDebtRequest},
        oracle::{self, PythId, WBTC_USD_ID},
    },
    grug::{
        btree_map, Addressable, Binary, Coins, ContractWrapper, Denom, HashExt, JsonSerExt,
        Message, MsgConfigure, NonEmpty, NumberConst, QuerierExt, ResultExt, Udec128, Uint128,
    },
    grug_app::NaiveProposalPreparer,
    std::{str::FromStr, sync::LazyLock},
};

static USDC: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("uusdc").unwrap());
static BTC: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("bridge/btc").unwrap());

/// The Pyth ID for USDC.
pub const USDC_USD_ID: &str = "0xeaa020c61cc479712813461ce153894a96a6c00b21ed0cfc2798d1f9a9e9c94a";

/// An example Pyth VAA for an USDC price feed.
/// - id: **eaa020c61cc479712813461ce153894a96a6c00b21ed0cfc2798d1f9a9e9c94a**
/// - price: **100000966**
/// - ema_price: **99999889**
/// - expo: **-8**
/// - publish_time: **1730802926**
const USDC_VAA: &str = "UE5BVQEAAAADuAEAAAAEDQOoMTxJ5BWLUCMy94ZlQ6qBjQEzA/+ZpDKw9AGFXXSyQF2eIKCGN6cNh1f/jzNSYOf15Yk2CRvOtMc7LqzdG7NpAQSNSaXe+ZOZU4+kxAgG74ZwDUuFmTPlElG90sIMNXfFmS6WJrbTBBQNWFL2gUKpdpEp5z/wUwJo/TzB9lHDnq2vAAbYj1fi3S3mzyOvZAPbe5Qy2/L/oQdLW4FPXTVcNxjMl1m0VLYRonpvIO4/S21ovvsefil9l8R3tYNG879aE2LMAQicgal5v2vVqicVvzE2J1vhg61mEvUKKhiZhzzWo8naRgQfuvKVk3257QhmGaDaAYWxU4MJ7goFUBPbBww9gk53AQpxhRMcpv+qmFMHZCdvoWwF4I/x230bO9VOQXie1tLSf25E62lWTAdYiyrh+h/ny7GA1aDLDZYwEzT6fXUPPlg/AAuQHWuf7TcUkOuIeVisiiI5XINdK8NFu36IacZjf0okOT9dApIx4sLAReROml2hs75v4a1K8SlLB3JdQkQLMoUDAQxZDK7Rh3UBSbjTrBKe+c+5lvT6ZgP26SOqF0F26xJIqwn29C8ZzCKkDgBNzx7GbA4bwL1tNNbv6NSxyx+72AlQAQ2+4nnWuPFUrn5dJJRD5VO6CYNu42Mx4XialbPJ6Lbp3gewVGOIIiU69PyeCxX6/Q/qO99Qtc+QlDGcyjmCwQP1AA7IcDlMiDVc4wEhkfCVRxCr//C3pGZsnxZguQr0MYaSnwGQ/FzJhBsU4knRtTZgUUm3rlcwNWDAJlp5MnNcPuYpAQ91tfYjBU0lRYDoYV/00L+RgJ66vx4P4T3R3x1MuDMAalgVHg43JfcUBGytMHWSbJr/24jMWMsEPMqwBuzPvba5ABDyPKTil7cKBdhyJhTJPYNS0V9JLbS6QLPCThaTyapMMW5BQfB07Q21fXnDNZE/FSoS4JxRiKcViiwRQ4lcLE/UARHcd8PSiHsEilgDjWOH/hvpaQ+Iza+rrBithaw+nJIIdClnizW0DqO2lVx0DlERwF8C9hL3hatj888kVWzwtj6RARJoRZtdYKzWJX8KzJvlOcOBxjjiCSyfo3qLfoLHIw2rJwT7HRxqg1wXswDjq2NVjms9jz24dRIEKM0dxfEP6OckAGcp9O4AAAAAABrhAfrtrFhR4yubI7X5QRqMK6xKrj7U3XuBHdGnLqSqcQAAAAAFb/IiAUFVV1YAAAAAAAp8vHgAACcQO03kFK+kZ552XKseu11fj2cpvpUBAFUA6qAgxhzEeXEoE0Yc4VOJSpamwAsh7Qz8J5jR+anpyUoAAAAABfXkxgAAAAAAAPrQ////+AAAAABnKfTuAAAAAGcp9O4AAAAABfXgkQAAAAAAAQTcCsjx5ZH7wLv7N+2Vzze0aT71EUmuA4n/zf/zQdrI6za/FR4xTLzViierrotGyMoKwkcBs++77xpXHT1p3YXWRMQCLxEONHC/rFMy+rS7i7XohTAftvazeHYjyF6a2rZNmf+KdZS2umZMH9qPKRD3USxGDnXfQMg9mgD6HwJnHiPgaublP56r5AqPcI1tyXKMfF10MWvyxkvJbXFUuYkzW0Pi03Asu75UoUT4XeKBXfvF+EL0NmKGNrmXDYH9NpT5H6pKDeS0JDCZ";

/// - id: **c9d8b075a5c69303365ae23633d4e085199bf5c520a3b90fed1322a0342ffc33**
/// - price: **7131950295749**
/// - publish_time: **1730209108**
const WBTC_VAA_1: &str = "UE5BVQEAAAADuAEAAAAEDQBLJRnF435tmWmnpCautCMOcWFhH0neObVk2iw/qtQ/jX44qUBV+Du+woo5lWLrE1ttnAPfwv9aftKy/r0pz0OdAQP25Bjy5Hx3MaOEF49sx+OrA6fxSNtBIxEkZ/wqznQAvlNE86loIz2osKoAWYeCg9FjU/8A2CmZZhcyXb4Cf+beAQSN829+7wKOw6tdMnKwtiYKdXL1yo1uP10iZ3EhU2M4cxrD0xYKA0pkb9hmhRo+zHrOY9pyTGXAsz7FjlI+gvgCAQa5MiGBgMRLFGW0fTd+bqc+isCQDbhgm/99yNkVaDt40ASST8CfH5zp4Xim5l5Yhs+/HMpeFSuTNULeDXsTO2FaAAjaPzeC8Bie6n154BaKA+45xn0lDa0epmVZs16zVCkKczSUNVG5e5VZe6N8edT+dVicoZYT9tgHJn2WDIjcpRv7AAsc0fdXE42zolp1Dhg1XVL5oe6NeTZi2Beu2ecv5FkvtCwm9dytTv6C359wJqUZLbZVaqOU9CEVbBvTzbKAm/tQAAx12qSCdkLtlJZAmhhrCvW56375q1Dy74L417r+GhDgYRqPCNWyaY7azRFfOwahxc9ECZgHj1aJg0bk395+JhTnAQ2K/IC6aRcSpPd+SfbWnfPtdJTdJFw5QCS50FbBfxxmqBTcG8E8fyYyCz5SGC8rtXgrBi+cQZe8FgW4CoLXXxC+AQ7TotPy0p9aHpwlIrXvu9B2nThByrwd4icwnOfQsUDHcG65PXWvu9nc1o5EK6SImnv+AmIu+RID2MnyTavsGEMpAA/XdQHG8mkgdWlZ1w7fg2MBs3fa0VxIlKc1DuaBdZVZEjrnB4gE15oqMZ21Bt8ji6r6J+ar/9K46EUeYC2t6CuBARDpRTI9ZZlh0MvxIbxRkuAgtRTv8oNrSz4sQJMNbhWdswTmqQQMZjtdJwGWepaAGhnEiuF/JgIr20AnDxCWbolgABGwVILVFDCHnLV54/bIdXUEiigPZvsKcDxLpOoJ722xZT1cXwXoBmwQ2lXQxGOjyj8VvgAt2kZJNbGc77+pmsqdABIFwK9Dc5BLxz+dXztA5bPMcEKkfZ18t7HPZ9BVQN7f1Cw4XcBZDSRR0MM6tqeBYvLJZhDMbt2Ax0m0+RlzQTZyAWcg5VQAAAAAABrhAfrtrFhR4yubI7X5QRqMK6xKrj7U3XuBHdGnLqSqcQAAAAAFWSo1AUFVV1YAAAAAAApl86sAACcQTdtYrFsURmdX9JeZM/nLGOdGy18BAFUAydiwdaXGkwM2WuI2M9TghRmb9cUgo7kP7RMioDQv/DMAAAZ8iV0qxQAAAAIvYnVX////+AAAAABnIOVUAAAAAGcg5VQAAAZ3rChYAAAAAAIykC3MCknCJZOvI3H3Ijt5NftDL77S253kTxg9ywpWvf3kzbZeQqXixw7K/fcAEWCww773jqhfS4CdRyUc38SMv+DhHywJbnUSyzFEWOTBVmVuvEtt6xWOTDMifAi8cAX0cBtZOyeIeLytWSqkMVYhtbm0gKCLnjtBEKLg/zEHSL48Ndm9VTihIpe8REto4Pf2MjlxRY6Smgw2TMZCJTCEj2869KzQsQhVSH4VmOJNJpevlYaqeFmJ7WDOC1tFWrVulGSZ/nIt63NKB+JP";

/// - id: **c9d8b075a5c69303365ae23633d4e085199bf5c520a3b90fed1322a0342ffc33**
/// - price: **9622018044290**
/// - publish_time: **1732965697**
const WBTC_VAA_2: &str = "UE5BVQEAAAADuAEAAAAEDQNHHIXSITl1E5rklfcRJ+fTmdXaBHA1Rhpd4AKd6gpJbGvcVZT0bmi5p1aaw10oEkruufXhvmEDgtCE1WENpk0xAARVRgaUYTxGBqPnzyOD6flYdHgL21qD2wm4AwHw8WMNOR3yvLDjuRkPdKCUHLUbOuaiUPJzK7DqphslPmTtChyXAQba+OsLHO3tJP5mFTfp9C0+wYbFk5DaXLYo0dHpshKqkC0tzXFrxzwiFUJscJ/qO5YC/ELvSB7eOyyYACqdzwJ+AQi+TpwsCHYMqT/DgSC3R4Il8WVVwp9KByaoAwswCH4y1mCtaV/rkpDHbK9dA9hm1hsEhpt3706y0YgttJny7uqCAQrPJK5440aRWGWqAUOH9COAomYlwb/qS+GJPRu5WizFIkhlsFK1osJSnHD5hmyXL0y58IyznYIjSr23/7RwWu//AAtm1B5zLdF2QgpQOLn3GGO8XNsvyWTb78I9oh1PQxnM+Rg89ImtRNpcbB9bF7wL3/wFUlZuBQGkm6igEzwzn/MGAAweFo1G9Fi0U9O9TjkUwGAKu4vVjwyP7j7tUI1FaYvRSxgL8Xvsx6uGePebx1y8V6TPcbjVR7ElL6TdKkOoVYxkAQ0M2X/klMnTmnBEZP1uMv4uAirn5QtFPX+q0FRI2Vnd/mPc1hF1saZ7kxah9M+V/1uGTQACalJBcLDLLbIcugy4AA5pvhB/AMUhVT5ejOO8ivazNhNtRhyp21Qk/qVYk8dmkhA+QY7RdjTXuU8TQg8e9HPhCo1/pNvt3NCZIP/ylNPHAQ+YF5H8kAq8OVTZlGvI7WvE6TKNtu8O9TuSdgibj5M3xFfZWf2hZZjuugKHVCe9DI0T+TIcjlyvON6+PrETUwNgARCfgdbK0GZ/Rw2GxakrXnE65vyUfMzTY63S93XcVrNlvSpEJIWbS2hhvxFw45WEksxPko6UfMJPpTZKG3sIAb4jABG1t35Wbn+AJ0fkoBPQL5lbRJVBTtY4RgBwhQnFKb2BdH8kHFI2DZD1pH+563Z08/RBPCjN48GZZzitp9OvBx8hABJTX8Up7HtYhLbqQendITTU3L27cmrGzjYHM7lR7VBlCBV7ebLHD8gShmxcJrPWnCCbuk6BrGRsTU7dgtz46U7jAGdJxewAAAAAABrhAfrtrFhR4yubI7X5QRqMK6xKrj7U3XuBHdGnLqSqcQAAAAAFv/gqAUFVV1YAAAAAAArM0q4AACcQvH/JSE5oq9bhIvQg1VV414fW/SsBAFUAydiwdaXGkwM2WuI2M9TghRmb9cUgo7kP7RMioDQv/DMAAAjRZEA9pwAAAAKQIm7x////+AAAAABnScXsAAAAAGdJxewAAAjPg8DAoAAAAAKaCFQ0CwdL0pYZ6jFIksWRnD8Yx1WX4LYMrhe9/+Z0T3i7MarKAlss59jV+mt6A/kKK+jDSP/oJz8vRNcCi8ZBhb/Qe7dJHJxIUzii5JHD9ItZ66YI1350NAVkQOysOWecA2JNOP1cK9RCHretlbv//OPlp7zfi8yn/wOr2RxcXPaERgM9r95qX6ltsOq8F6ZA45dR5DLrkby3Ymn1z+pLBYQWSubgJD5s8LNiBGKhME1OtZozdQZ9ifYY4FxdYRqBeiGgfunKrCJYh9Ud6LA+Xl+hRui8fmq8VIhmsxaPpPg=";

/// Asserts that two `Udec128` values are approximately equal within a specified
/// relative difference
fn assert_approx_eq(a: Udec128, b: Udec128, max_rel_diff: &str) {
    // Handle the case where both numbers are zero
    if a == Udec128::ZERO && b == Udec128::ZERO {
        return;
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
    let rel_diff = abs_diff / larger;

    // Assert that the relative difference is within the maximum allowed
    assert!(
        rel_diff <= Udec128::from_str(max_rel_diff).unwrap(),
        "assertion failed: values are not approximately equal\n  left: {}\n right: {}\n  max_rel_diff: {}\n  actual_rel_diff: {}",
        a,
        b,
        max_rel_diff,
        rel_diff
    );
}

fn register_price_feed(
    suite: &mut TestSuite<NaiveProposalPreparer>,
    accounts: &mut TestAccounts,
    contracts: &Contracts,
    denom: Denom,
    precision: u8,
    id: PythId,
) {
    // Register price source
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &dango_types::oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                denom => dango_types::oracle::PriceSource::Pyth { id, precision }
            }),
            Coins::default(),
        )
        .should_succeed();
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
            &oracle::ExecuteMsg::FeedPrices(NonEmpty::new_unchecked(vec![
                Binary::from_str(vaa).unwrap()
            ])),
            Coins::default(),
        )
        .should_succeed();
}

/// Feeds the oracle contract a price for USDC
fn register_and_feed_usdc_price(
    suite: &mut TestSuite<NaiveProposalPreparer>,
    accounts: &mut TestAccounts,
    contracts: &Contracts,
) {
    let id = PythId::from_str(USDC_USD_ID).unwrap();

    register_price_feed(suite, accounts, contracts, USDC.clone(), 6, id);
    feed_oracle_price(suite, accounts, contracts, USDC_VAA);
}

fn register_and_feed_btc_price(
    suite: &mut TestSuite<NaiveProposalPreparer>,
    accounts: &mut TestAccounts,
    contracts: &Contracts,
) {
    let btc_denom = Denom::from_str("bridge/btc").unwrap();

    register_price_feed(suite, accounts, contracts, btc_denom, 8, WBTC_USD_ID);
    feed_oracle_price(suite, accounts, contracts, WBTC_VAA_1);
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
fn margin_account_creation() {
    let (mut suite, mut accounts, codes, contracts) = setup_test_naive();

    // Create a margin account.
    accounts
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
        .should_succeed();
}

/// Some standard setup that needs to be done to get margin accounts working.
/// Does the following:
/// - feeds the oracle with a price for USDC (~$1) and WBTC (~$71K)
/// - creates a margin account
/// - deposits some USDC into the lending pool
/// - whitelists USDC as collateral at 100% power
/// - whitelists WBTC as collatearal at 80% power
/// - borrows from the margin account
fn setup_margin_test_env(
    suite: &mut TestSuite<NaiveProposalPreparer>,
    accounts: &mut Accounts,
    codes: &Codes<ContractWrapper>,
    contracts: &Contracts,
) -> TestAccount {
    register_and_feed_usdc_price(suite, accounts, contracts);
    register_and_feed_btc_price(suite, accounts, contracts);

    // Create a margin account.
    let margin_account = accounts
        .relayer
        .register_new_account(
            suite,
            contracts.account_factory,
            codes.account_margin.to_bytes().hash256(),
            AccountParams::Margin(single::Params {
                owner: accounts.relayer.username.clone(),
            }),
            Coins::new(),
        )
        .should_succeed();

    // Deposit some USDC to the lending pool
    suite
        .execute(
            &mut accounts.relayer,
            contracts.lending,
            &lending::ExecuteMsg::Deposit {},
            Coins::one(USDC.clone(), 100_000_000_000).unwrap(),
        )
        .should_succeed();

    // Whitelist USDC as collateral at 100% power
    set_collateral_power(
        suite,
        accounts,
        USDC.clone(),
        CollateralPower::new(Udec128::new_percent(100)).unwrap(),
    );

    // Add lending/borrowing market for BTC
    suite
        .execute(
            &mut accounts.owner,
            contracts.lending,
            &lending::ExecuteMsg::UpdateMarkets(btree_map! {
                BTC.clone() => MarketUpdates {},
            }),
            Coins::new(),
        )
        .should_succeed();

    // Deposit some btc to the lending pool
    suite
        .execute(
            &mut accounts.relayer,
            contracts.lending,
            &lending::ExecuteMsg::Deposit {},
            Coins::one(BTC.clone(), 100_000_000_000).unwrap(),
        )
        .should_succeed();

    // Whitelist WBTC as collateral at 80% power
    set_collateral_power(
        suite,
        accounts,
        BTC.clone(),
        CollateralPower::new(Udec128::new_percent(80)).unwrap(),
    );

    margin_account
}

#[test]
fn cant_liquidate_when_overcollateralised() {
    let (mut suite, mut accounts, codes, contracts) = setup_test_naive();
    let mut margin_account = setup_margin_test_env(&mut suite, &mut accounts, &codes, &contracts);

    // Borrow with the margin account
    suite
        .execute(
            &mut margin_account,
            contracts.lending,
            &lending::ExecuteMsg::Borrow(Coins::one(USDC.clone(), 100_000_000).unwrap()),
            Coins::new(),
        )
        .should_succeed();

    // Try to liquidate the margin account, should fail as it's not undercollateralized
    suite
        .execute(
            &mut accounts.relayer,
            margin_account.address(),
            &account::margin::ExecuteMsg::Liquidate {
                collateral: USDC.clone(),
            },
            Coins::new(),
        )
        .should_fail_with_error("account is not undercollateralized");
}

#[test]
fn liquidation_works() {
    let (mut suite, mut accounts, codes, contracts) = setup_test_naive();
    let mut margin_account = setup_margin_test_env(&mut suite, &mut accounts, &codes, &contracts);

    // Borrow with the margin account
    suite
        .execute(
            &mut margin_account,
            contracts.lending,
            &lending::ExecuteMsg::Borrow(Coins::one(USDC.clone(), 100_000_000).unwrap()),
            Coins::new(),
        )
        .should_succeed();

    // Confirm the margin account has the borrowed coins
    suite
        .query_balance(&margin_account.address(), USDC.clone())
        .should_succeed_and_equal(Uint128::new(100_000_000));

    // Update USDC collateral power to 90% to make the account undercollateralised
    set_collateral_power(
        &mut suite,
        &mut accounts,
        USDC.clone(),
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
        .query_balance(&accounts.relayer.address(), USDC.clone())
        .unwrap();

    // Try to partially liquidate the margin account, should succeed
    suite
        .execute(
            &mut accounts.relayer,
            margin_account.address(),
            &account::margin::ExecuteMsg::Liquidate {
                collateral: USDC.clone(),
            },
            Coins::one(USDC.clone(), 50_000_000).unwrap(),
        )
        .should_succeed();

    // Check liquidator account's USDC balance after
    let balance_after = suite
        .query_balance(&accounts.relayer.address(), USDC.clone())
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
        debts_before.amount_of(&USDC) - debts_after.amount_of(&USDC),
        Uint128::new(50_000_000)
    );

    // Try to liquidate the rest of the account's collateral, should succeed
    suite
        .execute(
            &mut accounts.relayer,
            margin_account.address(),
            &account::margin::ExecuteMsg::Liquidate {
                collateral: USDC.clone(),
            },
            Coins::one(USDC.clone(), 100_000_000).unwrap(),
        )
        .should_succeed();

    // Check liquidator account's USDC balance after
    let balance_before = balance_after;
    let balance_after = suite
        .query_balance(&accounts.relayer.address(), USDC.clone())
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
    assert!(debts_before.amount_of(&USDC) > debts_after.amount_of(&USDC));

    // Ensure the account has no collateral left
    suite
        .query_balance(&margin_account.address(), USDC.clone())
        .should_succeed_and_equal(Uint128::ZERO);
}

#[test]
fn liquidation_works_with_multiple_debt_denoms() {
    let (mut suite, mut accounts, codes, contracts) = setup_test_naive();
    let mut margin_account = setup_margin_test_env(&mut suite, &mut accounts, &codes, &contracts);

    // Borrow some USDC with the margin account
    suite
        .execute(
            &mut margin_account,
            contracts.lending,
            &lending::ExecuteMsg::Borrow(Coins::one(USDC.clone(), 1_000_000_000).unwrap()), // 1K USDC
            Coins::new(),
        )
        .should_succeed();

    // Send some more USDC to the margin account as collateral
    suite
        .transfer(
            &mut accounts.relayer,
            margin_account.address(),
            Coins::one(USDC.clone(), 15_000_000_000).unwrap(), // 15k USDC
        )
        .should_succeed();

    // Borrow some BTC
    suite
        .execute(
            &mut margin_account,
            contracts.lending,
            &lending::ExecuteMsg::Borrow(Coins::one(BTC.clone(), 100_000_000).unwrap()), // 1 BTC
            Coins::new(),
        )
        .should_succeed();

    // Query account's health
    let health = suite
        .query_wasm_smart(margin_account.address(), QueryHealthRequest {})
        .unwrap();
    assert!(health.utilization_rate < Udec128::ONE);

    // Update the oracle price of BTC to go from $71k to $96k, making the account undercollateralised
    feed_oracle_price(&mut suite, &mut accounts, &contracts, WBTC_VAA_2);

    // Query account's health
    let health = suite
        .query_wasm_smart(margin_account.address(), QueryHealthRequest {})
        .unwrap();
    assert!(health.utilization_rate > Udec128::ONE);
    let debts_before = health.debts;

    // Check liquidator account's USDC balance before
    let usdc_balance_before = suite
        .query_balance(&accounts.relayer.address(), USDC.clone())
        .unwrap();

    // Try to partially liquidate the margin account, fully paying off USDC debt,
    // and some of the BTC debt
    suite
        .execute(
            &mut accounts.relayer,
            margin_account.address(),
            &account::margin::ExecuteMsg::Liquidate {
                collateral: USDC.clone(),
            },
            Coins::try_from(btree_map! {
                "uusdc" => 1_000_000_000, // 1K USDC
                "bridge/btc" => 100_000, // 0.001 BTC
            })
            .unwrap(),
        )
        .should_succeed();

    // Check liquidator account's USDC balance after
    let usdc_balance_after = suite
        .query_balance(&accounts.relayer.address(), USDC.clone())
        .unwrap();

    // Ensure liquidators USDC balance increased
    assert!(usdc_balance_after > usdc_balance_before);

    // Account's debts should have decreased by the amount of the liquidation
    let debts_after = suite
        .query_wasm_smart(contracts.lending, QueryDebtRequest {
            account: margin_account.address(),
        })
        .unwrap();

    // Since this is a partial liquidation, ensure the debt has decreased exactly with the sent amount
    assert_eq!(
        debts_before.amount_of(&USDC) - debts_after.amount_of(&USDC),
        Uint128::new(1_000_000_000)
    );

    // Try to liquidate the rest of the account's BTC collateral, but send USDC
    // to cover the debt. Should fail since the account no longer has USDC debt.
    suite
        .execute(
            &mut accounts.relayer,
            margin_account.address(),
            &account::margin::ExecuteMsg::Liquidate {
                collateral: BTC.clone(),
            },
            Coins::one(USDC.clone(), 100_000_000).unwrap(),
        )
        .should_fail_with_error("no debt was repaid");

    // Check liquidator account's BTC balance before
    let btc_balance_before = suite
        .query_balance(&accounts.relayer.address(), BTC.clone())
        .unwrap();

    // Try to liquidate the rest of the account's collateral, should succeed
    suite
        .execute(
            &mut accounts.relayer,
            margin_account.address(),
            &account::margin::ExecuteMsg::Liquidate {
                collateral: BTC.clone(),
            },
            Coins::one(BTC.clone(), 100_000_000).unwrap(),
        )
        .should_succeed();

    // Check liquidator account's BTC balance after
    let btc_balance_after = suite
        .query_balance(&accounts.relayer.address(), BTC.clone())
        .unwrap();

    // Ensure balance increased (should receive collateral plus bonus worth more than the repaid debt)
    assert!(btc_balance_after > btc_balance_before);

    // Query account's health
    let health = suite
        .query_wasm_smart(margin_account.address(), QueryHealthRequest {})
        .unwrap();
    let app_config: AppConfig = suite.query_app_config().unwrap();

    // Ensure the new utilization rate is equal to the target utilization rate
    assert_approx_eq(
        health.utilization_rate,
        *app_config.target_utilization_rate,
        "0.0001",
    );

    // Check that the debt after is correct (using manual calculation via equations)
    assert_approx_eq(
        health.total_debt_value,
        Udec128::from_str("41609.67023").unwrap(),
        "0.0001",
    );
    let debts_after = suite
        .query_wasm_smart(contracts.lending, QueryDebtRequest {
            account: margin_account.address(),
        })
        .unwrap();
    assert_eq!(debts_after.amount_of(&BTC), Uint128::new(42916818));

    // Check that the collateral value after is correct (using manual calculation via equations)
    assert_approx_eq(
        health.total_adjusted_collateral_value,
        Udec128::from_str("46232.96693").unwrap(),
        "0.0001",
    );
}
