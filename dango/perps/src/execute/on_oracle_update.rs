use {
    crate::execute::ORACLE,
    anyhow::ensure,
    grug::{MutableCtx, Response},
};

/// Called once every block by the oracle contract after it receives updated prices.
///
/// Since validators feed price updates themselves and always pin the oracle
/// update transaction to the top of the block, this is guaranteed to happen as
/// the first thing each block.
pub fn on_oracle_update(ctx: MutableCtx) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ORACLE,
        "you don't have the right, O you don't have the right"
    );

    // TODO:
    // 1. accrue funding for all pairs
    // 2. check for unlocks that have completed cooldown and release the fund

    Ok(Response::new())
}
