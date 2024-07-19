use grug::{Empty, MutableCtx, Number, NumberConst, Response, StdResult, Uint128};

#[grug::export]
pub fn instantiate(_ctx: MutableCtx, _msg: Empty) -> StdResult<Response> {
    Ok(Response::new())
}

#[grug::export]
pub fn execute(_ctx: MutableCtx, _msg: Empty) -> StdResult<Response> {
    let mut number = Uint128::new(0);
    loop {
        number = number.wrapping_add(Uint128::ONE);
    }
}
