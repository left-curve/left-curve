use {
    crate::{FEE_SHARE_RATIO, PARAM, REFEREE_TO_REFERRER, query::query_volume},
    anyhow::{bail, ensure},
    dango_types::{
        DangoQuerier,
        account_factory::{self, UserIndex},
        perps::{FeeShareRatio, Referral},
    },
    grug::{MutableCtx, QuerierExt, Response},
};

/// Register a referral relationship between a referrer and a referee.
///
/// Caller must be either the account factory (during registration) or an
/// account owned by the referee.
pub fn set_referral(
    ctx: MutableCtx,
    referrer: UserIndex,
    referee: UserIndex,
) -> anyhow::Result<Response> {
    // Referrer and referee must be different users.
    ensure!(referrer != referee, "a user cannot refer themselves");

    // Caller must be the account factory or an account owned by the referee.
    let account_factory = ctx.querier.query_account_factory()?;

    if ctx.sender != account_factory {
        // If not the account factory, verify the sender is the referee.
        let account = ctx.querier.query_wasm_smart(
            account_factory,
            account_factory::QueryAccountRequest {
                address: ctx.sender,
            },
        )?;

        ensure!(
            account.owner == referee,
            "caller is not the account factory or the referee"
        );
    }

    // The referrer must have a share ratio set (i.e. has opted in as a referrer).
    ensure!(
        FEE_SHARE_RATIO.has(ctx.storage, referrer),
        "referrer {referrer} has no fee share ratio set"
    );

    // The referral relationship is immutable once set.
    REFEREE_TO_REFERRER.may_update(ctx.storage, referee, |existing| {
        if existing.is_some() {
            bail!("referee {referee} already has a referrer");
        }
        Ok(referrer)
    })?;

    Ok(Response::new().add_event(Referral { referrer, referee })?)
}

/// Set or update the fee share ratio for the calling user (referrer).
///
/// The share ratio can only increase, never decrease, once set.
/// The caller must have traded at least `volume_to_be_referrer` in lifetime
/// perps volume.
pub fn set_fee_share_ratio(
    ctx: MutableCtx,
    share_ratio: FeeShareRatio,
) -> anyhow::Result<Response> {
    // Look up the caller's user index via the account factory.
    let account_factory = ctx.querier.query_account_factory()?;

    let account =
        ctx.querier
            .query_wasm_smart(account_factory, account_factory::QueryAccountRequest {
                address: ctx.sender,
            })?;

    let user_index = account.owner;

    // The caller must have enough lifetime perps volume to become a referrer.
    let param = PARAM.load(ctx.storage)?;
    let volume = query_volume(ctx.storage, ctx.sender, None)?;

    ensure!(
        volume >= param.referral.volume_to_be_referrer,
        "insufficient perps volume to become a referrer (required: {}, current: {})",
        param.referral.volume_to_be_referrer,
        volume,
    );

    // If already set, the new ratio must be >= the existing one.
    if let Some(existing) = FEE_SHARE_RATIO.may_load(ctx.storage, user_index)? {
        ensure!(
            share_ratio >= existing,
            "fee share ratio can only increase (current: {existing}, proposed: {share_ratio})"
        );
    }

    FEE_SHARE_RATIO.save(ctx.storage, user_index, &share_ratio)?;

    Ok(Response::new())
}
