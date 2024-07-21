use grug::{
    to_json_value, Empty, ImmutableCtx, Json, MutableCtx, Number, Response, StdResult, Uint128,
};

#[grug::derive(serde)]
pub enum QueryMsg {
    /// Run a loop of the given number of iterations. Within each iteration, a
    /// set of math operations (addition, subtraction, multiplication, division)
    /// are performed.
    ///
    /// This is used for deducing the relation between Wasmer gas metering
    /// points and CPU time (i.e. how many gas points roughly correspond to one
    /// second of run time).
    Loop { iterations: u64 },
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(_ctx: MutableCtx, _msg: Empty) -> StdResult<Response> {
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(_ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::Loop { iterations } => to_json_value(&do_loop(iterations)?),
    }
}

// Function needs to be named `do_loop` instead of `loop`, because the latter is
// a reserved Rust keyword.
pub fn do_loop(iterations: u64) -> StdResult<Empty> {
    // Keep the same operation per iteration for consistency
    for _ in 0..iterations {
        let number = Uint128::new(100);
        number.checked_add(number)?;
        number.checked_sub(number)?;
        number.checked_mul(number)?;
        number.checked_div(number)?;
        number.checked_pow(2)?;
    }

    Ok(Empty {})
}
