use {
    crate::state::COMMISSION_RATE_OVERRIDES,
    anyhow::ensure,
    dango_types::{account_factory::UserIndex, perps::CommissionRate},
    grug::{MutableCtx, Op, QuerierExt, Response},
};

/// Set or remove a commission rate override for a user.
///
/// Only callable by the chain owner.
pub fn set_commission_rate_override(
    ctx: MutableCtx,
    user: UserIndex,
    commission_rate: Op<CommissionRate>,
) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "you don't have the right, O you don't have the right"
    );

    match commission_rate {
        Op::Insert(rate) => {
            COMMISSION_RATE_OVERRIDES.save(ctx.storage, user, &rate)?;
        },
        Op::Delete => {
            COMMISSION_RATE_OVERRIDES.remove(ctx.storage, user);
        },
    }

    Ok(Response::new())
}
