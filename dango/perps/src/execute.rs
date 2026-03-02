mod add_liquidity;
mod cancel_order;
mod deleverage;
mod deposit;
mod liquidate;
mod on_oracle_update;
mod remove_liquidity;
mod submit_order;
mod withdraw;

use {
    crate::{PAIR_IDS, PAIR_PARAMS, PAIR_STATES, PARAM, STATE},
    dango_types::{
        UsdValue,
        perps::{CancelOrderRequest, ExecuteMsg, InstantiateMsg, PairState, State},
    },
    grug::{Addr, MutableCtx, Response, Uint128, addr},
};

/// Virtual shares added to total supply in share price calculations.
/// Prevents the first-depositor attack (ERC-4626 inflation attack) by
/// ensuring the share price cannot be trivially inflated.
const VIRTUAL_SHARES: Uint128 = Uint128::new(1_000_000);

/// Virtual assets added to vault equity in share price calculations.
/// Works in tandem with `VIRTUAL_SHARES` to set the initial share price
/// and prevent share inflation attacks.
const VIRTUAL_ASSETS: UsdValue = UsdValue::new_int(1);

/// Address of the oracle contract.
pub(crate) const ORACLE: Addr = addr!("cedc5f73cbb963a48471b849c3650e6e34cd3b6d");

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    PARAM.save(ctx.storage, &msg.param)?;

    STATE.save(ctx.storage, &State {
        last_funding_time: ctx.block.timestamp,
        ..Default::default()
    })?;

    for (pair_id, pair_param) in &msg.pair_params {
        PAIR_PARAMS.save(ctx.storage, pair_id, pair_param)?;

        PAIR_STATES.save(ctx.storage, pair_id, &PairState::default())?;
    }

    PAIR_IDS.save(ctx.storage, &msg.pair_params.into_keys().collect())?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::Deposit {} => deposit::deposit(ctx),
        ExecuteMsg::Withdraw { margin } => withdraw::withdraw(ctx, margin),
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
        ExecuteMsg::AddLiquidity { min_shares_to_mint } => {
            add_liquidity::add_liquidity(ctx, min_shares_to_mint)
        },
        ExecuteMsg::RemoveLiquidity { shares_to_burn } => {
            remove_liquidity::remove_liquidity(ctx, shares_to_burn)
        },
        ExecuteMsg::Liquidate { user } => liquidate::liquidate(ctx, user),
        ExecuteMsg::Deleverage { user } => deleverage::deleverage(ctx, user),
        ExecuteMsg::OnOracleUpdate {} => on_oracle_update::on_oracle_update(ctx),
    }
}
