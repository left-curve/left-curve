use {
    crate::{ExecuteMsg, QueryForceWriteRequest},
    grug::{Coins, Message, MutableCtx, Number, NumberConst, Response, StdResult, Uint128},
};

pub fn infinite_loop() -> StdResult<Response> {
    let mut number = Uint128::new(0);
    loop {
        number = number.wrapping_add(Uint128::ONE);
    }
}

pub fn force_write_on_query(ctx: MutableCtx, key: String, value: String) -> StdResult<Response> {
    ctx.querier
        .query_wasm_smart(ctx.contract, QueryForceWriteRequest { key, value })?;

    Ok(Response::new())
}

pub fn exeucte_stack_overflow(ctx: MutableCtx) -> StdResult<Response> {
    Ok(Response::new().add_message(Message::execute(
        ctx.contract,
        &ExecuteMsg::StackOverflow {},
        Coins::default(),
    )?))
}
