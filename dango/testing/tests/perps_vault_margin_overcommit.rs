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
        Coins, Duration, NumberConst, QuerierExt, ResultExt, Timestamp, Udec128, Uint128, btree_map,
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
        liquidation_fee_rate: Dimensionless::new_permille(10), // 1%
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

/// Demonstrates a bug where the vault over-commits margin when sizing
/// market-making orders.
///
/// **Root cause** (`vault/refresh.rs:67`): `refresh_orders` uses the raw
/// `vault_state.margin` (deposited collateral) to size orders. It should use
/// available margin (`equity - used_margin`) which accounts for unrealized PnL
/// and margin consumed by existing positions.
///
/// **Current (buggy) behavior**:
/// After the vault accumulates positions from fills and the oracle price moves
/// against those positions, `refresh_orders` still sizes new orders based on
/// the full deposited margin. This causes the vault to place orders far larger
/// than its remaining equity can support. When a taker fills those orders, the
/// vault's total exposure grows until `equity < maintenance_margin` — an
/// unhealthy state that, for a normal user, would trigger liquidation.
///
/// **Expected behavior after fix**:
/// `refresh_orders` should compute the vault's available margin
/// (`equity - used_margin`). When available margin is zero or negative (as it
/// is after the price drop in this test), the vault should place no new orders,
/// keeping its position stable and its equity above the maintenance margin.
///
/// Scenario:
///   1. Vault receives $5,000 margin, market-makes ETH at $2,000
///   2. Taker fills vault bid -> vault goes long 12.5 ETH @ $1,980
///   3. Price drops to $1,700 -> equity=$1,500, available=$0, still healthy
///   4. Vault refreshes using raw margin=$5,000 -> places bid for ~14.7 ETH
///      (correct: available=$0, should place NO orders)
///   5. Taker fills that bid -> vault long ~27 ETH total
///   6. equity(~$1,750) < maintenance_margin(~$2,312) -> UNHEALTHY
///
/// This test asserts the vault is healthy at step 6 and is expected to FAIL
/// under the current code. Once the bug is fixed, this test should pass.
#[test]
fn vault_overcommits_margin_after_position_and_price_drop() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());

    let pair = pair_id();

    // Register oracle: ETH = $2,000, USDC = $1.
    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);

    // -------------------------------------------------------------------------
    // Step 1: LP (user1) deposits $5,000 USDC and adds all of it as vault
    // liquidity. After this the vault's margin = $5,000.
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

    let vault_state = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: contracts.perps,
        })
        .should_succeed()
        .unwrap();

    assert_eq!(vault_state.margin, UsdValue::new_int(5_000));

    // -------------------------------------------------------------------------
    // Step 2: Configure vault market-making.
    //   - vault_half_spread = 1%  -> bid at oracle*(1-1%), ask at oracle*(1+1%)
    //   - vault_max_quote_size = 100 ETH (large, won't constrain)
    //   - IMR = 10%, MMR = 5%
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
    // Step 3: Refresh vault orders (Round 1, ETH @ $2,000).
    //   half_margin = $2,500
    //   size = $2,500 / ($2,000 * 10%) = 12.5 ETH
    //   bid @ $2,000 * (1 - 1%) = $1,980
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

    assert_eq!(vault_bid.limit_price, UsdPrice::new_int(1_980));

    let round1_bid_size = vault_bid.size;

    // -------------------------------------------------------------------------
    // Step 4: Taker (user2) deposits $10,000 and market sells into vault bid.
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
                size: round1_bid_size.checked_neg().unwrap(),
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

    // Verify vault has a long position.
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

    // -------------------------------------------------------------------------
    // Step 5: Price drops to $1,700 (15% drop).
    //   Unrealized PnL = 12.5 * ($1,700 - $1,980) = -$3,500
    //   Equity = $5,000 - $3,500 = $1,500
    //   Used IM = 12.5 * $1,700 * 10% = $2,125
    //   Available = max(0, $1,500 - $2,125) = $0
    //   MM = 12.5 * $1,700 * 5% = $1,062.50
    //   Vault is STILL HEALTHY ($1,500 >= $1,062.50) but has $0 available.
    // -------------------------------------------------------------------------

    register_oracle_prices(&mut suite, &mut accounts, &contracts, 1_700);

    // Sanity check: vault should be healthy at this point.
    let vault_ext: perps::UserStateExtended = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateExtendedRequest {
            user: contracts.perps,
            include_equity: true,
            include_available_margin: true,
        })
        .should_succeed();

    let equity_before = vault_ext.equity.unwrap();
    let available_before = vault_ext.available_margin.unwrap();

    let vault_pos = vault_ext.raw.positions.get(&pair).unwrap();
    let mm_before = vault_pos
        .size
        .checked_abs()
        .unwrap()
        .checked_mul(UsdPrice::new_int(1_700))
        .unwrap()
        .checked_mul(Dimensionless::new_permille(50))
        .unwrap();

    assert!(
        equity_before >= mm_before,
        "vault should be healthy before Round 2: equity ({equity_before}) >= MM ({mm_before})"
    );
    assert_eq!(
        available_before,
        UsdValue::ZERO,
        "available margin should be zero — no room for new orders"
    );

    // -------------------------------------------------------------------------
    // Step 6: Refresh vault orders (Round 2, ETH @ $1,700) — THE BUG.
    //   Bug: uses raw margin ~$5,000 -> size = $2,500 / ($1,700 * 10%) = 14.7 ETH
    //   Correct: available = $0 -> should place NO orders.
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
        .expect("vault should have a bid (the bug — it shouldn't with correct logic)");

    let round2_bid_size = vault_bid.size;

    // -------------------------------------------------------------------------
    // Step 7: Taker fills the over-committed vault bid.
    //   Vault total position grows to ~27 ETH.
    // -------------------------------------------------------------------------

    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: round2_bid_size.checked_neg().unwrap(),
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

    // -------------------------------------------------------------------------
    // Step 8: Assert the vault is healthy — EXPECTED TO FAIL.
    //   After the over-committed fill, the vault's exposure exceeds what its
    //   equity can support: equity < maintenance_margin.
    // -------------------------------------------------------------------------

    let vault_ext: perps::UserStateExtended = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateExtendedRequest {
            user: contracts.perps,
            include_equity: true,
            include_available_margin: false,
        })
        .should_succeed();

    let equity = vault_ext.equity.unwrap();

    let vault_pos = vault_ext
        .raw
        .positions
        .get(&pair)
        .expect("vault should have a position");

    let maintenance_margin = vault_pos
        .size
        .checked_abs()
        .unwrap()
        .checked_mul(UsdPrice::new_int(1_700))
        .unwrap()
        .checked_mul(Dimensionless::new_permille(50)) // MMR = 5%
        .unwrap();

    assert!(
        equity >= maintenance_margin,
        "vault should be healthy: equity ({equity}) >= maintenance_margin ({maintenance_margin})"
    );
}
