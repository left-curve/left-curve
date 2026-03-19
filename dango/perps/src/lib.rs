mod core;
mod cron;
mod liquidity_depth;
mod maintain;
#[cfg(feature = "metrics")]
pub mod metrics;
mod position_index;
mod price;
mod querier;
mod query;
mod state;
mod trade;
mod vault;
mod volume;

pub(crate) use {querier::*, state::*, volume::*};

use {
    dango_oracle::OracleQuerier,
    dango_types::{
        DangoQuerier, UsdValue,
        perps::{
            CancelOrderRequest, ExecuteMsg, InstantiateMsg, MaintainerMsg, OrderId, QueryMsg,
            State, TraderMsg, VaultMsg,
        },
    },
    grug::{
        Addr, EventBuilder, ImmutableCtx, Json, JsonSerExt, MutableCtx, NumberConst, Response,
        SudoCtx, Uint128,
    },
};

/// Virtual shares added to total supply in share price calculations.
/// Prevents the first-depositor attack (ERC-4626 inflation attack) by
/// ensuring the share price cannot be trivially inflated.
const VIRTUAL_SHARES: Uint128 = Uint128::new(1_000_000);

/// Virtual assets added to vault equity in share price calculations.
/// Works in tandem with `VIRTUAL_SHARES` to set the initial share price
/// and prevent share inflation attacks.
const VIRTUAL_ASSETS: UsdValue = UsdValue::new_int(1);

/// Lookback window for volume-tiered fee rate resolution.
const VOLUME_LOOKBACK: grug::Duration = grug::Duration::from_days(14);

/// Returns the oracle contract address.
///
/// In release builds, returns a compile-time constant for zero-cost lookups.
/// In debug/test builds, queries the chain's `AppConfig` at runtime so that
/// tests with dynamically-derived addresses work correctly.
#[inline]
fn oracle(querier: impl DangoQuerier) -> Addr {
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

    maintain::configure(ctx, msg.param, msg.pair_params)
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn cron_execute(ctx: SudoCtx) -> anyhow::Result<Response> {
    #[cfg(feature = "metrics")]
    let start = std::time::Instant::now();

    let mut events = EventBuilder::new();

    cron::process_unlocks(ctx.storage, ctx.block.timestamp, &mut events)?;

    let mut oracle_querier = OracleQuerier::new_remote(oracle(ctx.querier), ctx.querier);

    cron::process_funding(ctx.storage, ctx.block.timestamp, &mut oracle_querier)?;

    #[cfg(feature = "metrics")]
    {
        let state = STATE.load(ctx.storage)?;
        let vault_user_state = USER_STATES
            .may_load(ctx.storage, ctx.contract)?
            .unwrap_or_default();

        let perp_querier = crate::NoCachePerpQuerier::new_local(ctx.storage);
        if let Ok(vault_equity) =
            crate::core::compute_user_equity(&mut oracle_querier, &perp_querier, &vault_user_state)
        {
            ::metrics::gauge!(crate::metrics::LABEL_VAULT_EQUITY)
                .set(crate::metrics::to_float(vault_equity));
        }

        ::metrics::gauge!(crate::metrics::LABEL_VAULT_MARGIN)
            .set(crate::metrics::to_float(vault_user_state.margin));

        for (pair_id, position) in &vault_user_state.positions {
            ::metrics::gauge!(crate::metrics::LABEL_VAULT_POSITION, "pair_id" => pair_id.to_string())
                .set(crate::metrics::to_float(position.size));
        }

        ::metrics::gauge!(crate::metrics::LABEL_INSURANCE_FUND)
            .set(crate::metrics::to_float(state.insurance_fund));

        ::metrics::gauge!(crate::metrics::LABEL_TREASURY)
            .set(crate::metrics::to_float(state.treasury));

        ::metrics::histogram!(crate::metrics::LABEL_DURATION_CRON)
            .record(start.elapsed().as_secs_f64());
    }

    Ok(Response::new().add_events(events)?)
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::Maintain(msg) => match msg {
            MaintainerMsg::Configure { param, pair_params } => {
                maintain::configure(ctx, param, pair_params)
            },
            MaintainerMsg::Liquidate { user } => maintain::liquidate(ctx, user),
        },
        ExecuteMsg::Trade(msg) => match msg {
            TraderMsg::Deposit {} => trade::deposit(ctx),
            TraderMsg::Withdraw { amount } => trade::withdraw(ctx, amount),
            TraderMsg::SubmitOrder {
                pair_id,
                size,
                kind,
                reduce_only,
            } => trade::submit_order(ctx, pair_id, size, kind, reduce_only),
            TraderMsg::CancelOrder(CancelOrderRequest::One(order_id)) => {
                trade::cancel_one_order(ctx, order_id)
            },
            TraderMsg::CancelOrder(CancelOrderRequest::All) => trade::cancel_all_orders(ctx),
        },
        ExecuteMsg::Vault(msg) => match msg {
            VaultMsg::AddLiquidity {
                amount,
                min_shares_to_mint,
            } => vault::add_liquidity(ctx, amount, min_shares_to_mint),
            VaultMsg::RemoveLiquidity { shares_to_burn } => {
                vault::remove_liquidity(ctx, shares_to_burn)
            },
            VaultMsg::Refresh {} => vault::refresh_orders(ctx),
        },
    }
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> anyhow::Result<Json> {
    match msg {
        QueryMsg::Param {} => PARAM.load(ctx.storage)?.to_json_value(),
        QueryMsg::PairParam { pair_id } => {
            PAIR_PARAMS.may_load(ctx.storage, &pair_id)?.to_json_value()
        },
        QueryMsg::PairParams { start_after, limit } => {
            query::query_pair_params(ctx, start_after, limit)?.to_json_value()
        },
        QueryMsg::State {} => STATE.load(ctx.storage)?.to_json_value(),
        QueryMsg::PairState { pair_id } => {
            PAIR_STATES.may_load(ctx.storage, &pair_id)?.to_json_value()
        },
        QueryMsg::PairStates { start_after, limit } => {
            query::query_pair_states(ctx, start_after, limit)?.to_json_value()
        },
        QueryMsg::UserState { user } => USER_STATES.may_load(ctx.storage, user)?.to_json_value(),
        QueryMsg::UserStates { start_after, limit } => {
            query::query_user_states(ctx, start_after, limit)?.to_json_value()
        },
        QueryMsg::Order { order_id } => query::query_order(ctx, order_id)?.to_json_value(),
        QueryMsg::OrdersByUser { user } => query::query_orders_by_user(ctx, user)?.to_json_value(),
        QueryMsg::LiquidityDepth {
            pair_id,
            bucket_size,
            limit,
        } => query::query_liquidity_depth(ctx, pair_id, bucket_size, limit)?.to_json_value(),
        QueryMsg::Volume { user, since } => {
            query::query_volume(ctx.storage, user, since)?.to_json_value()
        },
    }
    .map_err(Into::into)
}
