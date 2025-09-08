mod order_cancellation;
mod order_creation;

use {
    crate::{
        MAX_ORACLE_STALENESS, MINIMUM_LIQUIDITY, PAIRS, PAUSED, RESERVES,
        core::{self, PassiveLiquidityPool},
        cron,
    },
    anyhow::{anyhow, ensure},
    dango_oracle::OracleQuerier,
    dango_types::{
        DangoQuerier, bank,
        dex::{
            CallbackMsg, CancelOrderRequest, CreateOrderRequest, ExecuteMsg, InstantiateMsg,
            LP_NAMESPACE, NAMESPACE, OwnerMsg, PairId, PairUpdate, Paused, Swapped, Unpaused,
        },
        taxman::{self, FeeType},
    },
    grug::{
        Coin, CoinPair, Coins, DecCoins, Denom, EventBuilder, GENESIS_SENDER, Inner, IsZero,
        Message, MutableCtx, NonZero, QuerierExt, Response, Uint128, UniqueVec, btree_map, coins,
    },
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    PAUSED.save(ctx.storage, &false)?;
    batch_update_pairs(ctx, msg.pairs)
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::Owner(msg) => {
            // Only the chain owner can call owner functions.
            ensure!(
                ctx.sender == ctx.querier.query_owner()?,
                "you don't have the right, O you don't have the right"
            );

            match msg {
                OwnerMsg::SetPaused(true) => pause(ctx),
                OwnerMsg::SetPaused(false) => unpause(ctx),
                OwnerMsg::BatchUpdatePairs(updates) => batch_update_pairs(ctx, updates),
                OwnerMsg::ForceCancelOrders {} => force_cancel_orders(ctx),
            }
        },
        ExecuteMsg::Callback(msg) => {
            // Only the contract itself can call callback functions.
            ensure!(
                ctx.sender == ctx.contract,
                "you don't have the right, O you don't have the right"
            );

            match msg {
                CallbackMsg::Auction {} => cron::auction(ctx),
            }
        },
        ExecuteMsg::BatchUpdateOrders { creates, cancels } => {
            batch_update_orders(ctx, creates, cancels)
        },
        ExecuteMsg::ProvideLiquidity {
            base_denom,
            quote_denom,
        } => provide_liquidity(ctx, base_denom, quote_denom),
        ExecuteMsg::WithdrawLiquidity {
            base_denom,
            quote_denom,
        } => withdraw_liquidity(ctx, base_denom, quote_denom),
        ExecuteMsg::SwapExactAmountIn {
            route,
            minimum_output,
        } => swap_exact_amount_in(ctx, route.into_inner(), minimum_output),
        ExecuteMsg::SwapExactAmountOut { route, output } => {
            swap_exact_amount_out(ctx, route.into_inner(), output)
        },
    }
}

fn pause(ctx: MutableCtx) -> anyhow::Result<Response> {
    PAUSED.save(ctx.storage, &true)?;

    Ok(Response::new().add_event(Paused { error: None })?)
}

fn unpause(ctx: MutableCtx) -> anyhow::Result<Response> {
    PAUSED.save(ctx.storage, &false)?;

    Ok(Response::new().add_event(Unpaused {})?)
}

fn batch_update_pairs(ctx: MutableCtx, updates: Vec<PairUpdate>) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.querier.query_owner()? || ctx.sender == GENESIS_SENDER,
        "only the owner can update a trading pair parameters"
    );

    for update in updates {
        ensure!(
            update
                .params
                .lp_denom
                .starts_with(&[NAMESPACE.clone(), LP_NAMESPACE.clone()]),
            "LP token denom doesn't start with the correct prefix: `{}/{}/...`",
            NAMESPACE.as_ref(),
            LP_NAMESPACE.as_ref()
        );

        PAIRS.save(
            ctx.storage,
            (&update.base_denom, &update.quote_denom),
            &update.params,
        )?;
    }

    Ok(Response::new())
}

fn batch_update_orders(
    mut ctx: MutableCtx,
    creates: Vec<CreateOrderRequest>,
    cancels: Option<CancelOrderRequest>,
) -> anyhow::Result<Response> {
    // Creating or canceling orders is not allowed when the contract is paused.
    ensure!(
        !PAUSED.load(ctx.storage)?,
        "can't update orders when trading is paused"
    );

    let mut deposits = Coins::new();
    let mut refunds = DecCoins::new();
    let mut events = EventBuilder::new();

    match cancels {
        // Cancel selected orders.
        Some(CancelOrderRequest::Some(order_ids)) => {
            for order_id in order_ids {
                order_cancellation::cancel_order_from_user(
                    ctx.storage,
                    ctx.sender,
                    order_id,
                    &mut events,
                    &mut refunds,
                )?;
            }
        },
        // Cancel all orders.
        Some(CancelOrderRequest::All) => {
            order_cancellation::cancel_all_orders_from_user(
                ctx.storage,
                ctx.sender,
                &mut events,
                &mut refunds,
            )?;
        },
        // Do nothing.
        None => {},
    };

    for order in creates {
        order_creation::create_order(
            ctx.storage,
            ctx.block.height,
            ctx.sender,
            order,
            &mut events,
            &mut deposits,
        )?;
    }

    // Compute the amount of tokens that should be sent back to the users.
    //
    // This equals the amount that user has sent to the contract, plus the
    // amount that are to be refunded from the cancelled orders, and the amount
    // that the user is supposed to deposit for creating the new orders.
    ctx.funds
        .insert_many(refunds.into_coins_floor())?
        .deduct_many(deposits)
        .map_err(|e| anyhow!("insufficient funds for batch updating orders: {e}"))?;

    Ok(Response::new()
        .add_message(Message::transfer(ctx.sender, ctx.funds)?)
        .add_events(events)?)
}

fn provide_liquidity(
    mut ctx: MutableCtx,
    base_denom: Denom,
    quote_denom: Denom,
) -> anyhow::Result<Response> {
    // Get the deposited funds.
    let deposit = ctx
        .funds
        .take_pair((base_denom.clone(), quote_denom.clone()))?;

    // The user must have not sent any funds other the base/quote denoms.
    ensure!(
        ctx.funds.is_empty(),
        "unexpected deposit: {}; expecting `{}` and `{}`",
        ctx.funds,
        base_denom,
        quote_denom
    );

    // Load the pair params.
    let pair = PAIRS.load(ctx.storage, (&base_denom, &quote_denom))?;

    // Load the current pool reserve. Default to empty if not found.
    let reserve = RESERVES
        .may_load(ctx.storage, (&base_denom, &quote_denom))?
        .map_or_else(
            || CoinPair::new_empty(base_denom.clone(), quote_denom.clone()),
            Ok,
        )?;

    // Query the LP token supply.
    let lp_token_supply = ctx.querier.query_supply(pair.lp_denom.clone())?;

    // Create the oracle querier with max staleness.
    let mut oracle_querier = OracleQuerier::new_remote(ctx.querier.query_oracle()?, ctx.querier)
        .with_no_older_than(ctx.block.timestamp - MAX_ORACLE_STALENESS);

    // Compute the amount of LP tokens to mint.
    let (reserve, lp_mint_amount) =
        pair.add_liquidity(&mut oracle_querier, reserve, lp_token_supply, deposit)?;

    // Save the updated pool reserve.
    RESERVES.save(ctx.storage, (&base_denom, &quote_denom), &reserve)?;

    let bank = ctx.querier.query_bank()?;

    Ok(Response::new()
        .add_message(Message::execute(
            bank,
            &bank::ExecuteMsg::Mint {
                to: ctx.sender,
                coins: coins! { pair.lp_denom.clone() => lp_mint_amount },
            },
            Coins::new(), // No funds needed for minting
        )?)
        .may_add_message(if lp_token_supply.is_zero() {
            // If this is the first liquidity provision, mint a minimum liquidity.
            // to the contract itself and permanently lock it here. See the comment
            // on `MINIMUM_LIQUIDITY` for more details.
            //
            // Our implementation of this is slightly different from Uniswap's, which
            // mints 1000 tokens less to the user, while we mint 1000 extra to
            // the contract. Slightly different math but similarly prevents the
            // attack.
            Some(Message::execute(
                bank,
                &bank::ExecuteMsg::Mint {
                    to: ctx.contract,
                    coins: coins! { pair.lp_denom => MINIMUM_LIQUIDITY },
                },
                Coins::new(), // No funds needed for minting
            )?)
        } else {
            None
        }))
}

/// Withdraw liquidity from a pool. The LP tokens must be sent with the message.
/// The underlying assets will be returned to the sender.
fn withdraw_liquidity(
    mut ctx: MutableCtx,
    base_denom: Denom,
    quote_denom: Denom,
) -> anyhow::Result<Response> {
    // Load the pair params.
    let pair = PAIRS.load(ctx.storage, (&base_denom, &quote_denom))?;

    // Load the current pool reserve.
    let reserve = RESERVES.load(ctx.storage, (&base_denom, &quote_denom))?;

    // Query the LP token supply.
    let lp_token_supply = ctx.querier.query_supply(pair.lp_denom.clone())?;

    // Get the sent LP tokens.
    let lp_burn_amount = ctx.funds.take(pair.lp_denom.clone()).amount;

    // The user must have not sent any funds other the LP token.
    ensure!(
        ctx.funds.is_empty(),
        "unexpected deposit: {}; expecting `{}`",
        ctx.funds,
        pair.lp_denom
    );

    // Calculate the amount of each asset to return
    let (reserve, refunds) = pair.remove_liquidity(reserve, lp_token_supply, lp_burn_amount)?;

    // Save the updated pool reserve.
    RESERVES.save(ctx.storage, (&base_denom, &quote_denom), &reserve)?;

    Ok(Response::new()
        .add_message({
            let bank = ctx.querier.query_bank()?;
            Message::execute(
                bank,
                &bank::ExecuteMsg::Burn {
                    from: ctx.contract,
                    coins: coins! { pair.lp_denom => lp_burn_amount },
                },
                Coins::new(), // No funds needed for burning
            )?
        })
        .add_message(Message::transfer(ctx.sender, refunds)?))
}

fn swap_exact_amount_in(
    ctx: MutableCtx,
    route: UniqueVec<PairId>,
    minimum_output: Option<Uint128>,
) -> anyhow::Result<Response> {
    let input = ctx.funds.into_one_coin()?;
    let app_cfg = ctx.querier.query_dango_config()?;

    // Create the oracle querier with max staleness.
    let mut oracle_querier = OracleQuerier::new_remote(ctx.querier.query_oracle()?, ctx.querier)
        .with_no_older_than(ctx.block.timestamp - MAX_ORACLE_STALENESS);

    // Perform the swap.
    let (reserves, output, protocol_fee) = core::swap_exact_amount_in(
        ctx.storage,
        &mut oracle_querier,
        *app_cfg.taker_fee_rate, // Charge the taker fee rate for swaps.
        route,
        input.clone(),
    )?;

    // Ensure the output is above the minimum.
    // If not minimum is specified, the output should at least be greater than zero.
    if let Some(minimum_output) = minimum_output {
        ensure!(
            output.amount >= minimum_output,
            "output amount is below the minimum: {} < {}",
            output.amount,
            minimum_output
        );
    } else {
        ensure!(output.amount.is_non_zero(), "output amount is zero");
    }

    // Save the updated pool reserves.
    for (pair, reserve) in reserves {
        RESERVES.save(ctx.storage, (&pair.base_denom, &pair.quote_denom), &reserve)?;
    }

    Ok(Response::new()
        .add_message(Message::transfer(ctx.sender, output.clone())?)
        .may_add_message(if protocol_fee.is_non_zero() {
            Some(Message::execute(
                app_cfg.addresses.taxman,
                &taxman::ExecuteMsg::Pay {
                    ty: FeeType::Trade,
                    payments: btree_map! {
                        ctx.sender => coins! { output.denom.clone() => protocol_fee },
                    },
                },
                coins! { output.denom.clone() => protocol_fee },
            )?)
        } else {
            None
        })
        .add_event(Swapped {
            user: ctx.sender,
            input,
            output,
        })?)
}

fn swap_exact_amount_out(
    mut ctx: MutableCtx,
    route: UniqueVec<PairId>,
    output: NonZero<Coin>,
) -> anyhow::Result<Response> {
    let app_cfg = ctx.querier.query_dango_config()?;

    // Create the oracle querier with max staleness.
    let mut oracle_querier = OracleQuerier::new_remote(ctx.querier.query_oracle()?, ctx.querier)
        .with_no_older_than(ctx.block.timestamp - MAX_ORACLE_STALENESS);

    // Perform the swap.
    let (reserves, input, protocol_fee) = core::swap_exact_amount_out(
        ctx.storage,
        &mut oracle_querier,
        *app_cfg.taker_fee_rate, // Charge the taker fee rate for swaps.
        route,
        output.clone(),
    )?;

    // The user must have sent no less than the required input amount.
    // Any extra is refunded.
    ctx.funds
        .insert(output.clone().into_inner())?
        .deduct(input.clone())
        .map_err(|e| anyhow!("insufficient input for swap: {e}"))?;

    // Save the updated pool reserves.
    for (pair, reserve) in reserves {
        RESERVES.save(ctx.storage, (&pair.base_denom, &pair.quote_denom), &reserve)?;
    }

    Ok(Response::new()
        .add_message(Message::transfer(ctx.sender, ctx.funds)?)
        .may_add_message(if protocol_fee.is_non_zero() {
            Some(Message::execute(
                app_cfg.addresses.taxman,
                &taxman::ExecuteMsg::Pay {
                    ty: FeeType::Trade,
                    payments: btree_map! {
                        ctx.sender => coins! { output.denom.clone() => protocol_fee },
                    },
                },
                coins! { output.denom.clone() => protocol_fee },
            )?)
        } else {
            None
        })
        .add_event(Swapped {
            user: ctx.sender,
            input,
            output: output.into_inner(),
        })?)
}

fn force_cancel_orders(ctx: MutableCtx) -> anyhow::Result<Response> {
    let (events, refunds) = order_cancellation::cancel_all_orders(ctx.storage)?;

    Ok(Response::new()
        .add_events(events)?
        .add_message(refunds.into_message()))
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
// qlty-ignore: similar-code
mod tests {
    use {
        super::*,
        crate::RESTING_ORDER_BOOK,
        dango_types::{
            constants::{dango, usdc},
            dex::{
                AmountOption, PairParams, PassiveLiquidity, PriceOption, RestingOrderBookState,
                TimeInForce, Xyk,
            },
        },
        grug::{Addr, Bounded, MockContext, MockQuerier, NumberConst, Udec128, Udec128_24},
        std::{collections::BTreeSet, str::FromStr},
        test_case::test_case,
    };

    /// Ensure that if a user creates orders with more-than-sufficient funds, the
    /// extra funds are properly refunded.
    #[test_case(
        None,
        ExecuteMsg::BatchUpdateOrders {
            creates: vec![CreateOrderRequest {
                base_denom: dango::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
                price: PriceOption::Fixed(NonZero::new_unchecked(Udec128_24::new(2))),
                amount: AmountOption::Bid {
                    quote: NonZero::new_unchecked(Uint128::new(200)),
                },
                time_in_force: TimeInForce::GoodTilCanceled,
            }],
            cancels: None,
        },
        coins! { usdc::DENOM.clone() => 300 },
        coins! { usdc::DENOM.clone() => 100 };
        "overfunded limit bid: send 300, require 200"
    )]
    #[test_case(
        None,
        ExecuteMsg::BatchUpdateOrders {
            creates: vec![CreateOrderRequest {
                base_denom: dango::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
                price: PriceOption::Fixed(NonZero::new_unchecked(Udec128_24::new(2))),
                amount: AmountOption::Ask {
                    base: NonZero::new_unchecked(Uint128::new(100)),
                },
                time_in_force: TimeInForce::GoodTilCanceled,
            }],
            cancels: None,
        },
        coins! { dango::DENOM.clone() => 300 },
        coins! { dango::DENOM.clone() => 200 };
        "overfunded limit ask: send 300, require 100"
    )]
    #[test_case(
        Some(RestingOrderBookState {
            best_bid_price: Some(Udec128_24::new(100)),
            best_ask_price: Some(Udec128_24::new(100)),
            mid_price: Some(Udec128_24::new(100)),
        }),
        ExecuteMsg::BatchUpdateOrders {
            creates: vec![CreateOrderRequest {
                base_denom: dango::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
                price: PriceOption::BestAvailable {
                    max_slippage: Bounded::new_unchecked(Udec128::ZERO),
                },
                amount: AmountOption::Bid {
                    quote: NonZero::new_unchecked(Uint128::new(100)),
                },
                time_in_force: TimeInForce::ImmediateOrCancel,
            }],
            cancels: None,
        },
        coins! { usdc::DENOM.clone() => 300 },
        coins! { usdc::DENOM.clone() => 200 };
        "overfunded market bid: send 300, require 100"
    )]
    #[test_case(
        Some(RestingOrderBookState {
            best_bid_price: Some(Udec128_24::new(100)),
            best_ask_price: Some(Udec128_24::new(100)),
            mid_price: Some(Udec128_24::new(100)),
        }),
        ExecuteMsg::BatchUpdateOrders {
            creates: vec![CreateOrderRequest {
                base_denom: dango::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
                price: PriceOption::BestAvailable {
                    max_slippage: Bounded::new_unchecked(Udec128::ZERO),
                },
                amount: AmountOption::Ask {
                    base: NonZero::new_unchecked(Uint128::new(100)),
                },
                time_in_force: TimeInForce::ImmediateOrCancel,
            }],
            cancels: None,
        },
        coins! { dango::DENOM.clone() => 300 },
        coins! { dango::DENOM.clone() => 200 };
        "overfunded market ask: send 300, require 100"
    )]
    #[test_case(
        Some(RestingOrderBookState {
            best_bid_price: Some(Udec128_24::new(100)),
            best_ask_price: Some(Udec128_24::new(100)),
            mid_price: Some(Udec128_24::new(100)),
        }),
        ExecuteMsg::BatchUpdateOrders {
            creates: vec![
                // two market orders
                CreateOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    price: PriceOption::BestAvailable {
                        max_slippage: Bounded::new_unchecked(Udec128::ZERO),
                    },
                    amount: AmountOption::Bid {
                        quote: NonZero::new_unchecked(Uint128::new(100)),
                    },
                    time_in_force: TimeInForce::ImmediateOrCancel,
                },
                CreateOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    price: PriceOption::BestAvailable {
                        max_slippage: Bounded::new_unchecked(Udec128::ZERO),
                    },
                    amount: AmountOption::Ask {
                        base: NonZero::new_unchecked(Uint128::new(100)),
                    },
                    time_in_force: TimeInForce::ImmediateOrCancel,
                },
                // two limit orders
                CreateOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    price: PriceOption::Fixed(NonZero::new_unchecked(Udec128_24::new(2))),
                    amount: AmountOption::Bid {
                        quote: NonZero::new_unchecked(Uint128::new(200)),
                    },
                    time_in_force: TimeInForce::GoodTilCanceled,
                },
                CreateOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    price: PriceOption::Fixed(NonZero::new_unchecked(Udec128_24::new(2))),
                    amount: AmountOption::Ask {
                        base: NonZero::new_unchecked(Uint128::new(100)),
                    },
                    time_in_force: TimeInForce::GoodTilCanceled,
                },
            ],
            cancels: None,
        },
        coins! {
            usdc::DENOM.clone() => 600,
            dango::DENOM.clone() => 600,
        },
        coins! {
            usdc::DENOM.clone() => 300,
            dango::DENOM.clone() => 400,
        };
        "overfunded both in one tx; send 600 usdc + 600 dango, require 300 usdc + 200 dango"
    )]
    fn overfunded_order_refund_works(
        resting_order_book_state: Option<RestingOrderBookState>,
        msg: ExecuteMsg,
        funds: Coins,
        expected_refunds: Coins,
    ) {
        let sender = Addr::mock(1);
        let dex_contract = Addr::mock(2);

        let querier = MockQuerier::new().with_raw_contract_storage(dex_contract, |storage| {
            // Create the dango-usdc pair.
            // The specific parameters don't matter. We just need the pair to exist.
            PAIRS
                .save(storage, (&dango::DENOM, &usdc::DENOM), &PairParams {
                    lp_denom: Denom::from_str("lp").unwrap(),
                    pool_type: PassiveLiquidity::Xyk(Xyk {
                        spacing: Udec128::ONE,
                        reserve_ratio: Bounded::new_unchecked(Udec128::ZERO),
                        limit: 10,
                    }),
                    bucket_sizes: BTreeSet::new(),
                    swap_fee_rate: Bounded::new_unchecked(Udec128::new_bps(30)),
                    min_order_size: Uint128::ZERO,
                })
                .unwrap();
        });

        let mut ctx = MockContext::new()
            .with_contract(dex_contract)
            .with_sender(sender)
            .with_funds(funds)
            .with_querier(querier);

        // Set the pause state as unpaused.
        PAUSED.save(&mut ctx.storage, &false).unwrap();

        // Create the dango-usdc pair.
        // The specific parameters don't matter. We just need the pair to exist.
        PAIRS
            .save(
                &mut ctx.storage,
                (&dango::DENOM, &usdc::DENOM),
                &PairParams {
                    lp_denom: Denom::from_str("lp").unwrap(),
                    pool_type: PassiveLiquidity::Xyk(Xyk {
                        spacing: Udec128::ONE,
                        reserve_ratio: Bounded::new_unchecked(Udec128::ZERO),
                        limit: 10,
                    }),
                    bucket_sizes: BTreeSet::new(),
                    swap_fee_rate: Bounded::new_unchecked(Udec128::new_bps(30)),
                    min_order_size: Uint128::ZERO,
                },
            )
            .unwrap();

        // If a resting order book state is provided, set it up.
        // This is needed if the test case involves market orders. If limit orders
        // only then not needed.
        if let Some(resting_order_book_state) = resting_order_book_state {
            RESTING_ORDER_BOOK
                .save(
                    &mut ctx.storage,
                    (&dango::DENOM, &usdc::DENOM),
                    &resting_order_book_state,
                )
                .unwrap();
        }

        // The response should contain exactly 1 message, which is the refund.
        let res = execute(ctx.as_mutable(), msg).unwrap();
        assert_eq!(res.submsgs.len(), 1);
        assert_eq!(
            res.submsgs[0].msg,
            Message::Transfer(btree_map! { sender => expected_refunds })
        );
    }
}
