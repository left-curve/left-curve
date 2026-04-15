use {
    crate::{default_pair_param, default_param, refresh_vault_orders, register_oracle_prices},
    dango_testing::{TestOption, perps::pair_id, setup_test_naive},
    dango_types::{
        Dimensionless, Quantity, UsdPrice, UsdValue,
        constants::usdc,
        perps::{self, PairParam, Param, QueryOrdersByUserResponseItem},
    },
    grug::{Addressable, Coins, NumberConst, QuerierExt, ResultExt, Uint128, btree_map},
    std::collections::BTreeMap,
};

/// Regression test: vault liquidity withdrawal can push the vault below its
/// maintenance margin, making it immediately liquidatable.
///
/// The bug: `_remove_liquidity` checks `vault_user_state.margin >= amount_to_release`
/// (raw margin only), but does NOT verify the vault remains above maintenance
/// margin post-withdrawal. When the vault has unrealized losses, its raw margin
/// is much higher than its equity. The withdrawal amount is based on equity
/// (smaller), so the raw margin check passes -- but the margin deduction reduces
/// equity below maintenance margin.
///
/// This is the same class of bug that `add_liquidity` already fixed (see
/// `add_liquidity_rejects_when_available_margin_insufficient` unit test in
/// `dango/perps/src/vault/add_liquidity.rs`).
///
/// | Step | Action                                          | Key numbers                                                    | Assert                           |
/// | ---- | ----------------------------------------------- | -------------------------------------------------------------- | -------------------------------- |
/// | 1    | LP deposits $5k, adds $5k vault liquidity       | vault margin=$5,000; ~5B shares minted                         | shares > 0, vault margin = $5k   |
/// | 2    | Configure vault MM (5% spread, max 2 ETH)       | bid=$1,900, ask=$2,100                                         | —                                |
/// | 3    | Refresh vault orders                            | vault places bid + ask                                         | vault bid at $1,900              |
/// | 4    | Taker sells 2 ETH into vault bid                | vault long 2 ETH @ $1,900; margin still $5,000                | vault has long position          |
/// | 5    | Oracle drops to $1,500                          | PnL=−$800; equity=$4,200; MM=$150                              | vault healthy (equity > MM)      |
/// | 6    | LP burns ALL shares                             | release~$4,200; margin check $5k>=$4.2k passes; equity→~$0    | should_succeed (the bug)         |
/// | 7    | Assert vault is liquidatable                    | equity~$0 < MM=$150                                            | equity < MM                      |
/// | 8    | Bidder places bid 2 ETH @ $1,500                | provides book liquidity for liquidation                        | —                                |
/// | 9    | Liquidate vault                                 | closes vault's long against bidder                             | liquidation succeeds             |
#[test]
fn vault_withdrawal_makes_vault_liquidatable() {
    // ---- Step 0: Setup ----

    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    let pair = pair_id();

    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);

    // -------------------------------------------------------------------------
    // Step 1: LP (user1) deposits $5,000 USDC and adds $5,000 as vault liquidity.
    // -------------------------------------------------------------------------

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
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

    let lp_state: perps::UserState = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed()
        .unwrap();
    let lp_shares = lp_state.vault_shares;
    assert!(lp_shares > Uint128::ZERO, "LP should have vault shares");

    let vault_state: perps::UserState = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: contracts.perps,
        })
        .should_succeed()
        .unwrap();
    assert_eq!(
        vault_state.margin,
        UsdValue::new_int(5_000),
        "vault margin should be $5,000"
    );

    // -------------------------------------------------------------------------
    // Step 2: Configure vault market-making: 5% half-spread, max 2 ETH per side.
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
                        vault_half_spread: Dimensionless::new_permille(50), // 5%
                        vault_max_quote_size: Quantity::new_int(2),
                        ..default_pair_param()
                    },
                },
            }),
            Coins::new(),
        )
        .should_succeed();

    // -------------------------------------------------------------------------
    // Step 3: Refresh vault orders. Expect bid at $1,900 (= $2,000 * 0.95).
    // -------------------------------------------------------------------------

    suite.make_empty_block();
    refresh_vault_orders(&mut suite, &mut accounts, &contracts);

    let vault_orders: BTreeMap<perps::OrderId, QueryOrdersByUserResponseItem> = suite
        .query_wasm_smart(contracts.perps, perps::QueryOrdersByUserRequest {
            user: contracts.perps,
        })
        .should_succeed();

    let vault_bids: Vec<_> = vault_orders
        .values()
        .filter(|o| o.size.is_positive())
        .collect();
    assert!(
        !vault_bids.is_empty(),
        "vault should have a bid on the book"
    );
    assert_eq!(
        vault_bids[0].limit_price,
        UsdPrice::new_int(1_900),
        "vault bid should be at $1,900"
    );

    let vault_bid_size = vault_bids[0].size;

    // -------------------------------------------------------------------------
    // Step 4: Taker (user2) deposits $10,000 and market sells into vault's bid.
    //
    // Vault goes long 2 ETH at $1,900. As maker, vault pays zero maker fee,
    // so vault margin stays at $5,000.
    // -------------------------------------------------------------------------

    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: vault_bid_size.checked_neg().unwrap(), // sell
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::new_percent(10),
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    // Verify vault has a long position and margin is unchanged.
    let vault_state: perps::UserState = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: contracts.perps,
        })
        .should_succeed()
        .unwrap();
    let vault_pos = vault_state
        .positions
        .get(&pair)
        .expect("vault should have ETH position");
    assert!(vault_pos.size.is_positive(), "vault should be long ETH");
    // Vault margin is $5,000 + taker fee received as maker.
    // Exact value doesn't matter for this test; the key point is that raw
    // margin is significantly higher than equity after the oracle drop.
    assert!(
        vault_state.margin >= UsdValue::new_int(5_000),
        "vault margin should be at least $5,000"
    );

    // -------------------------------------------------------------------------
    // Step 5: Oracle drops to $1,500.
    //
    // Unrealized PnL = 2 * ($1,500 - $1,900) = -$800
    // Vault equity  = $5,000 + (-$800) = $4,200
    // MM            = 2 * $1,500 * 5%  = $150
    // Vault is healthy: $4,200 > $150.
    // -------------------------------------------------------------------------

    register_oracle_prices(&mut suite, &mut accounts, &contracts, 1_500);

    let vault_ext: perps::UserStateExtended = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateExtendedRequest {
            user: contracts.perps,
            include_equity: true,
            include_maintenance_margin: true,
            include_available_margin: false,
            include_unrealized_pnl: false,
            include_unrealized_funding: false,
            include_liquidation_price: false,
            include_all: false,
        })
        .should_succeed();

    let equity_before = vault_ext.equity.unwrap();
    let mm_before = vault_ext.maintenance_margin.unwrap();
    assert!(
        equity_before > mm_before,
        "vault should be healthy BEFORE withdrawal: equity={equity_before}, MM={mm_before}"
    );

    // -------------------------------------------------------------------------
    // Step 6: LP tries to burn ALL shares.
    //
    // amount_to_release ≈ $4,204 (equity-proportional, based on share ratio).
    // Old raw-margin check would pass: $5,000 >= $4,204.
    //
    // Fix: the available-margin check uses equity minus initial margin (IMR),
    // which is ~$3,904 — less than the $4,204 release amount. Rejected.
    // -------------------------------------------------------------------------

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Vault(perps::VaultMsg::RemoveLiquidity {
                shares_to_burn: lp_shares,
            }),
            Coins::new(),
        )
        .should_fail_with_error("insufficient vault available margin to cover withdrawal");
}
