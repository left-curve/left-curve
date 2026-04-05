use {
    dango_genesis::Contracts,
    dango_testing::{TestOption, perps::pair_id, setup_test_naive},
    dango_types::{
        Dimensionless, Quantity, UsdPrice, UsdValue,
        constants::usdc,
        oracle::{self, PriceSource},
        perps::{self, PairParam, Param, QueryOrdersByUserResponseItem},
    },
    grug::{
        Addressable, Coins, Duration, NumberConst, QuerierExt, ResultExt, Timestamp, Udec128,
        Uint128, btree_map,
    },
    std::collections::BTreeMap,
};

// ---------------------------------------------------------------------------
// Helpers (duplicated from perps.rs — test crates cannot share non-pub items)
// ---------------------------------------------------------------------------

fn default_param() -> Param {
    Param {
        taker_fee_rates: perps::RateSchedule {
            base: Dimensionless::new_permille(1), // 0.1%
            ..Default::default()
        },
        protocol_fee_rate: Dimensionless::ZERO,
        // Zero so the post-close fee doesn't erode equity below MM,
        // allowing us to assert post-liquidation health.
        liquidation_fee_rate: Dimensionless::ZERO,
        vault_cooldown_period: Duration::from_days(1),
        max_unlocks: 10,
        max_open_orders: 100,
        funding_period: Duration::from_hours(1),
        ..Default::default()
    }
}

fn default_pair_param() -> PairParam {
    PairParam {
        initial_margin_ratio: Dimensionless::new_permille(100), // 10%
        maintenance_margin_ratio: Dimensionless::new_permille(50), // 5%
        tick_size: UsdPrice::new_int(1),
        max_abs_oi: Quantity::new_int(1_000_000),
        ..PairParam::new_mock()
    }
}

fn register_oracle_prices(
    suite: &mut dango_testing::TestSuite<grug_app::NaiveProposalPreparer>,
    accounts: &mut dango_testing::TestAccounts,
    contracts: &Contracts,
    eth_price: u128,
) {
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                usdc::DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::ONE,
                    precision: usdc::DECIMAL as u8,
                    timestamp: Timestamp::from_nanos(u128::MAX),
                },
                pair_id() => PriceSource::Fixed {
                    humanized_price: Udec128::new(eth_price),
                    precision: 0,
                    timestamp: Timestamp::from_nanos(u128::MAX),
                },
            }),
            Coins::new(),
        )
        .should_succeed();
}

// ---------------------------------------------------------------------------
// Test
// ---------------------------------------------------------------------------

/// Verifies that the vault (contract address) can be liquidated when its
/// equity falls below maintenance margin.
///
/// The vault accumulates a losing long position through market-making, then a
/// price drop makes it liquidatable. A keeper calls `Liquidate` and the vault's
/// position is closed against resting bids on the order book.
///
/// Scenario:
///   1. LP deposits $5,000 → vault margin = $5,000
///   2. Vault refreshes at ETH=$2,000 → places 12.5 ETH bid at $1,980
///   3. Taker fills bid → vault long 12.5 ETH @ $1,980
///   4. Price drops to $1,600:
///      - Unrealized PnL = 12.5 × ($1,600 − $1,980) = −$4,750
///      - Equity = $5,000 − $4,750 = $250
///      - MM = 12.5 × $1,600 × 5% = $1,000
///      - $250 < $1,000 → liquidatable
///   5. Bidder places limit bid at $1,600 (provides book liquidity)
///   6. Liquidation closes vault's long against the bid
///   7. Assert: vault positions cleared, insurance fund received fee,
///      vault margin adjusted by PnL
#[test]
fn vault_liquidation_on_order_book() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());

    let pair = pair_id();

    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);

    // -------------------------------------------------------------------------
    // Step 1: LP (user1) deposits $5,000 USDC and adds all as vault liquidity.
    // -------------------------------------------------------------------------

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit {}),
            Coins::one(usdc::DENOM.clone(), Uint128::new(5_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Vault(perps::VaultMsg::AddLiquidity {
                amount: UsdValue::new_int(5_000),
                min_shares_to_mint: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    // -------------------------------------------------------------------------
    // Step 2: Configure vault market-making.
    // -------------------------------------------------------------------------

    suite
        .execute(
            &mut accounts.owner,
            contracts.perps,
            &perps::ExecuteMsg::Maintain(perps::MaintainerMsg::Configure {
                param: Param {
                    vault_total_weight: Dimensionless::new_int(1),
                    ..default_param()
                },
                pair_params: btree_map! {
                    pair.clone() => PairParam {
                        vault_liquidity_weight: Dimensionless::new_int(1),
                        vault_half_spread: Dimensionless::new_permille(10), // 1%
                        vault_max_quote_size: Quantity::new_int(100),
                        ..default_pair_param()
                    },
                },
            }),
            Coins::new(),
        )
        .should_succeed();

    // -------------------------------------------------------------------------
    // Step 3: Refresh vault orders → vault places bid at $1,980 for 12.5 ETH.
    // -------------------------------------------------------------------------

    suite
        .execute(
            &mut accounts.owner,
            contracts.perps,
            &perps::ExecuteMsg::Vault(perps::VaultMsg::Refresh {}),
            Coins::new(),
        )
        .should_succeed();

    let vault_orders: BTreeMap<perps::OrderId, QueryOrdersByUserResponseItem> = suite
        .query_wasm_smart(contracts.perps, perps::QueryOrdersByUserRequest {
            user: contracts.perps,
        })
        .should_succeed();

    let vault_bid = vault_orders
        .values()
        .find(|o| o.size.is_positive())
        .expect("vault should have a bid");

    let bid_size = vault_bid.size;

    // -------------------------------------------------------------------------
    // Step 4: Taker (user2) deposits and market sells into vault bid.
    //   Vault goes long 12.5 ETH @ $1,980.
    // -------------------------------------------------------------------------

    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit {}),
            Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: bid_size.checked_neg().unwrap(),
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::ONE,
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    // Verify vault is long.
    let vault_state = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: contracts.perps,
        })
        .should_succeed()
        .unwrap();

    let vault_pos = vault_state
        .positions
        .get(&pair)
        .expect("vault should have a position");
    assert!(vault_pos.size.is_positive(), "vault should be long");

    let vault_margin_before = vault_state.margin;

    // -------------------------------------------------------------------------
    // Step 5: Price drops to $1,600 (20% drop). Vault becomes liquidatable.
    //   equity = $5,000 + 12.5 * ($1,600 - $1,980) = $250
    //   MM = 12.5 * $1,600 * 5% = $1,000
    //   $250 < $1,000 → liquidatable
    // -------------------------------------------------------------------------

    register_oracle_prices(&mut suite, &mut accounts, &contracts, 1_600);

    // Sanity: verify vault is liquidatable (equity < MM).
    let vault_ext: perps::UserStateExtended = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateExtendedRequest {
            user: contracts.perps,
            include_equity: true,
            include_available_margin: false,
        })
        .should_succeed();

    let equity = vault_ext.equity.unwrap();
    let vault_pos = vault_ext.raw.positions.get(&pair).unwrap();
    let mm = vault_pos
        .size
        .checked_abs()
        .unwrap()
        .checked_mul(UsdPrice::new_int(1_600))
        .unwrap()
        .checked_mul(Dimensionless::new_permille(50))
        .unwrap();

    assert!(
        equity < mm,
        "vault should be liquidatable: equity ({equity}) < MM ({mm})"
    );

    // Record insurance fund before liquidation.
    let global_before: perps::State = suite
        .query_wasm_smart(contracts.perps, perps::QueryStateRequest {})
        .should_succeed();

    let insurance_before = global_before.insurance_fund;

    // -------------------------------------------------------------------------
    // Step 6: Bidder (user3) deposits and places limit bid at $1,600 for the
    //   vault's full position size. This provides liquidity for the liquidation.
    // -------------------------------------------------------------------------

    suite
        .execute(
            &mut accounts.user3,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit {}),
            Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user3,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: bid_size, // buy same size as vault's long
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(1_600),
                    post_only: true,
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    // -------------------------------------------------------------------------
    // Step 7: Liquidate the vault.
    // -------------------------------------------------------------------------

    suite
        .execute(
            &mut accounts.owner,
            contracts.perps,
            &perps::ExecuteMsg::Maintain(perps::MaintainerMsg::Liquidate {
                user: contracts.perps,
            }),
            Coins::new(),
        )
        .should_succeed();

    // -------------------------------------------------------------------------
    // Step 8: Assertions.
    // -------------------------------------------------------------------------

    // Vault position should be reduced (partial liquidation closes just enough
    // to restore equity above maintenance margin).
    let vault_state_after = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: contracts.perps,
        })
        .should_succeed()
        .unwrap();

    let vault_pos_after = vault_state_after.positions.get(&pair);
    let position_reduced = match vault_pos_after {
        None => true,                                                // fully closed
        Some(pos) => pos.size < vault_ext.raw.positions[&pair].size, // partially closed
    };

    assert!(
        position_reduced,
        "vault position should be reduced by liquidation"
    );

    // Vault margin should be reduced (PnL loss).
    let vault_margin_after = vault_state_after.margin;

    assert!(
        vault_margin_after < vault_margin_before,
        "vault margin should decrease: before={vault_margin_before}, after={vault_margin_after}"
    );

    // Vault should be healthy after liquidation (equity >= MM).
    // This holds because liquidation_fee_rate = 0, so no fee erodes the
    // buffer created by the close schedule.
    let vault_ext_after: perps::UserStateExtended = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateExtendedRequest {
            user: contracts.perps,
            include_equity: true,
            include_available_margin: false,
        })
        .should_succeed();

    let equity_after = vault_ext_after.equity.unwrap();
    let mm_after = match vault_ext_after.raw.positions.get(&pair) {
        Some(pos) => pos
            .size
            .checked_abs()
            .unwrap()
            .checked_mul(UsdPrice::new_int(1_600))
            .unwrap()
            .checked_mul(Dimensionless::new_permille(50))
            .unwrap(),
        None => UsdValue::ZERO,
    };

    assert!(
        equity_after >= mm_after,
        "vault should be healthy after liquidation: equity ({equity_after}) >= MM ({mm_after})"
    );

    // Insurance fund should be unchanged (liquidation_fee_rate = 0).
    let global_after: perps::State = suite
        .query_wasm_smart(contracts.perps, perps::QueryStateRequest {})
        .should_succeed();

    assert_eq!(
        global_after.insurance_fund, insurance_before,
        "insurance fund should be unchanged with zero liquidation fee"
    );

    // Bidder (user3) should now have a long position from absorbing the vault's close.
    let user3_state = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user3.address(),
        })
        .should_succeed()
        .unwrap();

    let user3_pos = user3_state
        .positions
        .get(&pair)
        .expect("user3 should have a position");
    assert!(
        user3_pos.size.is_positive(),
        "user3 should be long (absorbed vault's sell)"
    );
}
