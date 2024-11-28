use {
    crate::{NEXT_POSITION_INDEX, POSITIONS, UNLOCKING_SCHEDULE},
    anyhow::{bail, ensure},
    dango_types::vesting::{
        ExecuteMsg, InstantiateMsg, Position, PositionIndex, Schedule, VestingStatus,
    },
    grug::{Addr, Coin, IsZero, Message, MutableCtx, Response},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    UNLOCKING_SCHEDULE.save(ctx.storage, &Schedule {
        start_time: ctx.block.timestamp,
        cliff: msg.unlocking_cliff,
        vesting: msg.unlocking_vesting,
    })?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::CreatePosition { user, schedule } => create_position(ctx, user, schedule),
        ExecuteMsg::Claim { idx } => claim(ctx, idx),
        ExecuteMsg::TerminatePosition { idx } => terminate_position(ctx, idx),
    }
}

fn create_position(ctx: MutableCtx, user: Addr, schedule: Schedule) -> anyhow::Result<Response> {
    let amount = ctx.funds.into_one_coin()?;
    let (_, index) = NEXT_POSITION_INDEX.increment(ctx.storage)?;
    let position = Position::new(user, schedule, amount);

    POSITIONS.save(ctx.storage, index, &position)?;

    Ok(Response::new())
}

fn claim(ctx: MutableCtx, idx: PositionIndex) -> anyhow::Result<Response> {
    let mut position = POSITIONS.load(ctx.storage, idx)?;

    ensure!(
        position.user == ctx.sender,
        "you don't have the right, O you don't have the right"
    );

    let unlocking_schedule = UNLOCKING_SCHEDULE.load(ctx.storage)?;

    let claimable_amount =
        position.compute_claimable_amount(ctx.block.timestamp, &unlocking_schedule)?;

    ensure!(!claimable_amount.is_zero(), "nothing to claim");

    position.claimed_amount += claimable_amount;

    if position.full_claimed() {
        POSITIONS.remove(ctx.storage, idx)?;
    } else {
        POSITIONS.save(ctx.storage, idx, &position)?;
    }

    Ok(Response::new().add_message(Message::transfer(
        ctx.sender,
        Coin::new(position.vested_token.denom, claimable_amount)?,
    )?))
}

fn terminate_position(ctx: MutableCtx, idx: PositionIndex) -> anyhow::Result<Response> {
    let cfg = ctx.querier.query_config()?;

    ensure!(
        cfg.owner == ctx.sender,
        "you don't have the right, O you don't have the right"
    );

    let mut position = POSITIONS.load(ctx.storage, idx)?;

    let terminal_amount = if let VestingStatus::Active(schedule) = &position.vesting_status {
        schedule.compute_claimable_amount(ctx.block.timestamp, position.vested_token.amount)?
    } else {
        bail!("position is already terminated")
    };

    position.vesting_status = VestingStatus::Terminated(terminal_amount);

    let refund_amount = position.vested_token.amount - terminal_amount;

    let refund_msg = if refund_amount.is_zero() {
        None
    } else {
        Some(Message::transfer(
            cfg.owner,
            Coin::new(position.vested_token.denom.clone(), refund_amount)?,
        )?)
    };

    if position.claimed_amount == terminal_amount {
        POSITIONS.remove(ctx.storage, idx)?;
    } else {
        POSITIONS.save(ctx.storage, idx, &position)?;
    }

    Ok(Response::new().may_add_message(refund_msg))
}
