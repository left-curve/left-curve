use {
    crate::state::FEE_RATE_OVERRIDES,
    anyhow::ensure,
    dango_types::Dimensionless,
    grug::{Addr, MutableCtx, Op, QuerierExt, Response},
};

pub fn set_fee_rate_override(
    ctx: MutableCtx,
    user: Addr,
    maker_taker_fee_rates: Op<(Dimensionless, Dimensionless)>,
) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "you don't have the right, O you don't have the right"
    );

    match maker_taker_fee_rates {
        Op::Insert((maker_fee_rate, taker_fee_rate)) => {
            ensure!(
                (Dimensionless::ZERO..=Dimensionless::ONE).contains(&maker_fee_rate),
                "invalid maker fee rate: {maker_fee_rate}! must be within [0, 1]"
            );

            ensure!(
                (Dimensionless::ZERO..=Dimensionless::ONE).contains(&taker_fee_rate),
                "invalid taker fee rate: {taker_fee_rate}! must be within [0, 1]"
            );

            FEE_RATE_OVERRIDES.save(ctx.storage, user, &(maker_fee_rate, taker_fee_rate))?;
        },
        Op::Delete => {
            FEE_RATE_OVERRIDES.remove(ctx.storage, user);
        },
    }

    Ok(Response::new())
}
