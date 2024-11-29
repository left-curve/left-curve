use {
    crate::{POSITIONS, UNLOCKING_SCHEDULE},
    anyhow::{bail, ensure},
    dango_types::{
        config::AppConfig,
        vesting::{ExecuteMsg, InstantiateMsg, Position, Schedule, VestingStatus},
    },
    grug::{Addr, Coin, IsZero, Message, MutableCtx, Number, Response},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    UNLOCKING_SCHEDULE.save(ctx.storage, &Schedule {
        start_time: ctx.block.timestamp,
        cliff: msg.unlocking_cliff,
        period: msg.unlocking_period,
    })?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::Create { user, schedule } => create(ctx, user, schedule),
        ExecuteMsg::Terminate { user } => terminate(ctx, user),
        ExecuteMsg::Claim {} => claim(ctx),
    }
}

fn create(ctx: MutableCtx, user: Addr, schedule: Schedule) -> anyhow::Result<Response> {
    let cfg: AppConfig = ctx.querier.query_app_config()?;
    let coin = ctx.funds.into_one_coin_of_denom(&cfg.dango)?;
    let position = Position::new(schedule, coin.amount);

    POSITIONS.save(ctx.storage, user, &position)?;

    Ok(Response::new())
}

fn terminate(ctx: MutableCtx, user: Addr) -> anyhow::Result<Response> {
    let cfg = ctx.querier.query_config()?;
    let app_cfg: AppConfig = ctx.querier.query_app_config()?;

    ensure!(
        cfg.owner == ctx.sender,
        "you don't have the right, O you don't have the right"
    );

    let mut position = POSITIONS.load(ctx.storage, user)?;

    let terminal_amount = if let VestingStatus::Active(schedule) = &position.vesting_status {
        schedule.compute_claimable_amount(ctx.block.timestamp, position.total_amount)?
    } else {
        bail!("position is already terminated")
    };

    position.vesting_status = VestingStatus::Terminated(terminal_amount);

    let refund_amount = position.total_amount - terminal_amount;

    let refund_msg = if refund_amount.is_zero() {
        None
    } else {
        Some(Message::transfer(
            cfg.owner,
            Coin::new(app_cfg.dango, refund_amount)?,
        )?)
    };

    if position.claimed_amount == terminal_amount {
        POSITIONS.remove(ctx.storage, user);
    } else {
        POSITIONS.save(ctx.storage, user, &position)?;
    }

    Ok(Response::new().may_add_message(refund_msg))
}

fn claim(ctx: MutableCtx) -> anyhow::Result<Response> {
    let cfg: AppConfig = ctx.querier.query_app_config()?;
    let mut position = POSITIONS.load(ctx.storage, ctx.sender)?;

    let unlocking_schedule = UNLOCKING_SCHEDULE.load(ctx.storage)?;

    let claimable_amount =
        position.compute_claimable_amount(ctx.block.timestamp, &unlocking_schedule)?;

    ensure!(!claimable_amount.is_zero(), "nothing to claim");

    position
        .claimed_amount
        .checked_add_assign(claimable_amount)?;

    if position.full_claimed() {
        POSITIONS.remove(ctx.storage, ctx.sender);
    } else {
        POSITIONS.save(ctx.storage, ctx.sender, &position)?;
    }

    Ok(Response::new().add_message(Message::transfer(
        ctx.sender,
        Coin::new(cfg.dango, claimable_amount)?,
    )?))
}
