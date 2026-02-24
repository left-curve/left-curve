use {
    crate::execute::ORACLE,
    anyhow::ensure,
    grug::{MutableCtx, Response},
};

pub fn on_oracle_update(ctx: MutableCtx) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ORACLE,
        "you don't have the right, O you don't have the right"
    );

    // TODO:
    // 1. accrue funding for all pairs
    // 2. check for fillable limit orders and fill them
    // 3. check for unlocks that have completed cooldown and release the fund

    Ok(Response::new())
}
