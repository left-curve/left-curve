mod order_cancellation;
mod order_creation;

use {
    crate::{
        MAX_ORACLE_STALENESS, MINIMUM_LIQUIDITY, PAIRS, PAUSED, RESERVES, RESTING_ORDER_BOOK,
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
        Message, MutableCtx, NonZero, Number, QuerierExt, Response, Uint128, UniqueVec, btree_map,
        coins,
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
                OwnerMsg::BatchUpdatePairs(updates) => batch_update_pairs(ctx, updates),
                OwnerMsg::SetPaused(true) => pause(ctx),
                OwnerMsg::SetPaused(false) => unpause(ctx),
                OwnerMsg::Reset {} => reset(ctx),
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
            minimum_output,
        } => provide_liquidity(ctx, base_denom, quote_denom, minimum_output),
        ExecuteMsg::WithdrawLiquidity {
            base_denom,
            quote_denom,
            minimum_output,
        } => withdraw_liquidity(ctx, base_denom, quote_denom, minimum_output),
        ExecuteMsg::SwapExactAmountIn {
            route,
            minimum_output,
        } => swap_exact_amount_in(ctx, route.into_inner(), minimum_output),
        ExecuteMsg::SwapExactAmountOut { route, output } => {
            swap_exact_amount_out(ctx, route.into_inner(), output)
        },
    }
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

fn pause(ctx: MutableCtx) -> anyhow::Result<Response> {
    PAUSED.save(ctx.storage, &true)?;

    Ok(Response::new().add_event(Paused { error: None })?)
}

fn unpause(ctx: MutableCtx) -> anyhow::Result<Response> {
    PAUSED.save(ctx.storage, &false)?;

    Ok(Response::new().add_event(Unpaused {})?)
}

fn reset(ctx: MutableCtx) -> anyhow::Result<Response> {
    let (events, refunds) = order_cancellation::cancel_all_orders(ctx.storage, ctx.contract)?;

    RESTING_ORDER_BOOK.clear(ctx.storage, None, None);

    Ok(Response::new()
        .add_events(events)?
        .add_message(refunds.into_message()))
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
                    ctx.contract,
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
                ctx.contract,
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
    minimum_output: Option<Uint128>,
) -> anyhow::Result<Response> {
    // Providing liquidity is not allowed when trading is paused.
    ensure!(
        !PAUSED.load(ctx.storage)?,
        "can't provide liquidity when trading is paused"
    );

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

    // Get the taker fee rate from the app config.
    let app_cfg = ctx.querier.query_dango_config()?;

    // Compute the amount of LP tokens to mint and any protocol fees to collect.
    let (reserve, mut lp_mint_amount, protocol_fee_amount) = pair.add_liquidity(
        &mut oracle_querier,
        reserve,
        lp_token_supply,
        deposit,
        *app_cfg.taker_fee_rate,
    )?;

    // Subtract minimum liquidity from the mint amount if this is the first
    // liquidity provision.
    // See the comment on `MINIMUM_LIQUIDITY` on why this is necessary.
    if lp_token_supply.is_zero() {
        lp_mint_amount
            .checked_sub_assign(MINIMUM_LIQUIDITY)
            .map_err(|err| {
                anyhow!("LP token mint amount is less than `MINIMUM_LIQUIDITY`: {err}")
            })?;
    }

    // Ensure the LP mint amount is greater than the minimum.
    if let Some(minimum) = minimum_output {
        ensure!(
            lp_mint_amount >= minimum,
            "LP mint amount is less than the minimum output: {lp_mint_amount} < {minimum}"
        );
    }

    // Save the updated pool reserve.
    RESERVES.save(ctx.storage, (&base_denom, &quote_denom), &reserve)?;

    let bank = ctx.querier.query_bank()?;

    // Convert protocol fee CoinPair to Coins for payment
    let protocol_fee_coins = coins! { pair.lp_denom.clone() => protocol_fee_amount };

    let protocol_fee_msgs = if protocol_fee_amount.is_non_zero() {
        vec![
            Message::execute(
                bank,
                &bank::ExecuteMsg::Mint {
                    to: ctx.contract,
                    coins: protocol_fee_coins.clone(),
                },
                Coins::new(), // No funds needed for minting
            )?,
            Message::execute(
                app_cfg.addresses.taxman,
                &taxman::ExecuteMsg::Pay {
                    ty: FeeType::Trade,
                    payments: btree_map! {
                        ctx.sender => protocol_fee_coins.clone(),
                    },
                },
                protocol_fee_coins,
            )?,
        ]
    } else {
        vec![]
    };

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
        })
        .add_messages(protocol_fee_msgs))
}

/// Withdraw liquidity from a pool. The LP tokens must be sent with the message.
/// The underlying assets will be returned to the sender.
fn withdraw_liquidity(
    mut ctx: MutableCtx,
    base_denom: Denom,
    quote_denom: Denom,
    minimum_output: Option<CoinPair>,
) -> anyhow::Result<Response> {
    // Withdrawing liquidity is not allowed when trading is paused.
    ensure!(
        !PAUSED.load(ctx.storage)?,
        "can't withdraw liquidity when trading is paused"
    );

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

    // If a minimum output is specified, ensure the refunds are no less than it.
    if let Some(minimum_output) = minimum_output {
        ensure!(
            {
                let first = minimum_output.first();
                let second = minimum_output.second();
                refunds.amount_of(first.denom)? >= *first.amount
                    && refunds.amount_of(second.denom)? >= *second.amount
            },
            "withdrawn assets are less than the minimum output: {refunds:?} < {minimum_output:?}",
        );
    }

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
    // Swapping is not allowed when trading is paused.
    ensure!(
        !PAUSED.load(ctx.storage)?,
        "can't swap when trading is paused"
    );

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
    // Swapping is not allowed when trading is paused.
    ensure!(
        !PAUSED.load(ctx.storage)?,
        "can't swap when trading is paused"
    );

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
                AmountOption, PairParams, PassiveLiquidity, Price, PriceOption,
                RestingOrderBookState, TimeInForce, Xyk,
            },
        },
        grug::{Addr, Bounded, MockContext, MockQuerier, NumberConst, Udec128},
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
                price: PriceOption::Limit(NonZero::new_unchecked(Price::new(2))),
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
                price: PriceOption::Limit(NonZero::new_unchecked(Price::new(2))),
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
            best_bid_price: Some(Price::new(100)),
            best_ask_price: Some(Price::new(100)),
            mid_price: Some(Price::new(100)),
        }),
        ExecuteMsg::BatchUpdateOrders {
            creates: vec![CreateOrderRequest {
                base_denom: dango::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
                price: PriceOption::Market {
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
            best_bid_price: Some(Price::new(100)),
            best_ask_price: Some(Price::new(100)),
            mid_price: Some(Price::new(100)),
        }),
        ExecuteMsg::BatchUpdateOrders {
            creates: vec![CreateOrderRequest {
                base_denom: dango::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
                price: PriceOption::Market {
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
            best_bid_price: Some(Price::new(100)),
            best_ask_price: Some(Price::new(100)),
            mid_price: Some(Price::new(100)),
        }),
        ExecuteMsg::BatchUpdateOrders {
            creates: vec![
                // two market orders
                CreateOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    price: PriceOption::Market {
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
                    price: PriceOption::Market {
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
                    price: PriceOption::Limit(NonZero::new_unchecked(Price::new(2))),
                    amount: AmountOption::Bid {
                        quote: NonZero::new_unchecked(Uint128::new(200)),
                    },
                    time_in_force: TimeInForce::GoodTilCanceled,
                },
                CreateOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    price: PriceOption::Limit(NonZero::new_unchecked(Price::new(2))),
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
                    min_order_size_quote: Uint128::ZERO,
                    min_order_size_base: Uint128::ZERO,
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
                    min_order_size_quote: Uint128::ZERO,
                    min_order_size_base: Uint128::ZERO,
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
