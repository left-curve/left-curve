use {
    crate::{CONFIG, NEXT_POSITION_INDEX, POSITIONS},
    anyhow::{bail, ensure},
    dango_types::vesting::{
        Config, ExecuteMsg, InstantiateMsg, Position, PositionIndex, Schedule, VestingStatus,
    },
    grug::{Addr, Coin, Duration, IsZero, Message, MutableCtx, Response},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    CONFIG.save(ctx.storage, &Config {
        owner: msg.owner,
        unlocking_schedule: msg.unlocking_schedule.set_start_time(ctx.block.timestamp)?,
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

fn create_position(
    ctx: MutableCtx,
    user: Addr,
    schedule: Schedule<Option<Duration>>,
) -> anyhow::Result<Response> {
    let amount = ctx.funds.into_one_coin()?;
    let (_, index) = NEXT_POSITION_INDEX.increment(ctx.storage)?;
    let schedule = schedule.set_start_time(ctx.block.timestamp)?;
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

    let unlocking_schedule = CONFIG.load(ctx.storage)?.unlocking_schedule;

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
    let config = CONFIG.load(ctx.storage)?;

    ensure!(
        config.owner == ctx.sender,
        "you don't have the right, O you don't have the right"
    );

    let mut position = POSITIONS.load(ctx.storage, idx)?;

    let terminated_amount = if let VestingStatus::Active(schedule) = &position.vesting_status {
        schedule.compute_claimable_amount(ctx.block.timestamp, position.vested_token.amount)?
    } else {
        bail!("position is already terminated")
    };

    position.vesting_status = VestingStatus::Terminated(terminated_amount);

    let refund_amount = position.vested_token.amount - terminated_amount;

    let maybe_msg = if refund_amount.is_zero() {
        None
    } else {
        Some(Message::transfer(
            config.owner,
            Coin::new(position.vested_token.denom.clone(), refund_amount)?,
        )?)
    };

    if position.claimed_amount == terminated_amount {
        POSITIONS.remove(ctx.storage, idx)?;
    } else {
        POSITIONS.save(ctx.storage, idx, &position)?;
    }

    Ok(Response::new().may_add_message(maybe_msg))
}
