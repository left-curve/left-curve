use {
    crate::{NEXT_POSITION_INDEX, POSITIONS},
    anyhow::ensure,
    dango_types::vesting::{ExecuteMsg, InstantiateMsg, Position, PositionIndex, Schedule},
    grug::{Addr, Coin, Duration, IsZero, Message, MutableCtx, Response, StdResult},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(_ctx: MutableCtx, _msg: InstantiateMsg) -> StdResult<Response> {
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::CreatePosition { user, schedule } => create_position(ctx, user, schedule),
        ExecuteMsg::Claim { idx } => claim(ctx, idx),
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

    let claimable_amount = position.compute_claimable_amount(ctx.block.timestamp)?;

    ensure!(
        !claimable_amount.is_zero(),
        "don't try to claim twice in the same block"
    );

    position.claimed_amount += claimable_amount;

    if position.claimed_amount == position.amount.amount {
        POSITIONS.remove(ctx.storage, idx)?;
    } else {
        POSITIONS.save(ctx.storage, idx, &position)?;
    }

    Ok(Response::new().add_message(Message::transfer(
        ctx.sender,
        Coin::new(position.amount.denom, claimable_amount)?,
    )?))
}
