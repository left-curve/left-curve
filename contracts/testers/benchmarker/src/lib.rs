use grug::{grug_derive, grug_export, MutableCtx, Number, Response, StdResult, Uint128};

#[grug_derive(serde)]
pub struct TestConfig {
    pub iterations: u64,
    pub debug: bool,
}

fn do_iteration(ctx: MutableCtx, config: TestConfig) -> StdResult<Response> {
    if config.debug {
        ctx.api.debug(&ctx.contract, &config.iterations.to_string());
    }

    for i in 0..config.iterations {
        if config.debug {
            ctx.api.debug(&ctx.contract, &i.to_string());
        }

        // keep the same operation per iteration
        let number = Uint128::new(100);
        number.checked_add(number)?;
        number.checked_sub(number)?;
        number.checked_mul(number)?;
        number.checked_div(number)?;
        number.checked_pow(2)?;
    }
    Ok(Response::default())
}

#[grug_export]
pub fn instantiate(ctx: MutableCtx, iteration: TestConfig) -> StdResult<Response> {
    do_iteration(ctx, iteration)
}

#[grug_export]
pub fn execute(ctx: MutableCtx, iteration: TestConfig) -> StdResult<Response> {
    do_iteration(ctx, iteration)
}
