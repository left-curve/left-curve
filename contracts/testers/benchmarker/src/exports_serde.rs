use grug::{grug_export, MutableCtx, Response, StdResult};

use crate::{execute::do_test, types::ExecuteTest};

#[grug_export]
pub fn instantiate(ctx: MutableCtx, test: ExecuteTest) -> StdResult<Response> {
    do_test(ctx, test)
}

#[grug_export]
pub fn execute(ctx: MutableCtx, test: ExecuteTest) -> StdResult<Response> {
    do_test(ctx, test)
}
