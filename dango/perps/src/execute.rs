mod add_liquidity;
mod cancel_order;
mod configure;
mod deposit;
mod liquidate;
mod on_oracle_update;
mod remove_liquidity;
mod submit_order;
mod withdraw;

use {
    crate::{NEXT_ORDER_ID, STATE},
    dango_types::{
        DangoQuerier, UsdValue,
        perps::{CancelOrderRequest, ExecuteMsg, InstantiateMsg, OrderId, State},
    },
    grug::{Addr, MutableCtx, NumberConst, Response, Uint128},
};

/// Virtual shares added to total supply in share price calculations.
/// Prevents the first-depositor attack (ERC-4626 inflation attack) by
/// ensuring the share price cannot be trivially inflated.
const VIRTUAL_SHARES: Uint128 = Uint128::new(1_000_000);

/// Virtual assets added to vault equity in share price calculations.
/// Works in tandem with `VIRTUAL_SHARES` to set the initial share price
/// and prevent share inflation attacks.
const VIRTUAL_ASSETS: UsdValue = UsdValue::new_int(1);

/// Returns the oracle contract address.
///
/// In release builds, returns a compile-time constant for zero-cost lookups.
/// In debug/test builds, queries the chain's `AppConfig` at runtime so that
/// tests with dynamically-derived addresses work correctly.
#[inline]
pub(crate) fn oracle(querier: impl DangoQuerier) -> Addr {
    #[cfg(not(debug_assertions))]
    {
        let _ = querier;
        grug::addr!("cedc5f73cbb963a48471b849c3650e6e34cd3b6d")
    }

    #[cfg(debug_assertions)]
    {
        querier.query_oracle().unwrap()
    }
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    STATE.save(ctx.storage, &State {
        last_funding_time: ctx.block.timestamp,
        ..Default::default()
    })?;

    NEXT_ORDER_ID.save(ctx.storage, &OrderId::ONE)?;

    configure::configure(ctx, msg.param, msg.pair_params)
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::Deposit {} => deposit::deposit(ctx),
        ExecuteMsg::Withdraw { amount } => withdraw::withdraw(ctx, amount),
        ExecuteMsg::SubmitOrder {
            pair_id,
            size,
            kind,
            reduce_only,
        } => submit_order::submit_order(ctx, pair_id, size, kind, reduce_only),
        ExecuteMsg::CancelOrder(CancelOrderRequest::One(order_id)) => {
            cancel_order::cancel_one_order(ctx, order_id)
        },
        ExecuteMsg::CancelOrder(CancelOrderRequest::All) => cancel_order::cancel_all_orders(ctx),
        ExecuteMsg::AddLiquidity {
            amount,
            min_shares_to_mint,
        } => add_liquidity::add_liquidity(ctx, amount, min_shares_to_mint),
        ExecuteMsg::RemoveLiquidity { shares_to_burn } => {
            remove_liquidity::remove_liquidity(ctx, shares_to_burn)
        },
        ExecuteMsg::Liquidate { user } => liquidate::liquidate(ctx, user),
        ExecuteMsg::Configure { param, pair_params } => {
            configure::configure(ctx, param, pair_params)
        },
        ExecuteMsg::OnOracleUpdate {} => on_oracle_update::on_oracle_update(ctx),
    }
}
