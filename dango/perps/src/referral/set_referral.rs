use {
    crate::{
        account_factory,
        referral::load_referral_data,
        state::{
            FEE_SHARE_RATIO, REFEREE_TO_REFERRER, REFERRER_TO_REFEREE_STATISTICS,
            USER_REFERRAL_DATA,
        },
        volume::round_to_day,
    },
    anyhow::ensure,
    dango_types::{
        account_factory::{self, UserIndex},
        perps::{RefereeStats, ReferralSet},
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
    let account_factory = account_factory(ctx.querier);

    if ctx.sender != account_factory {
        // If not the account factory, verify the sender is the referee.
        // TODO: refactor to raw query (query_wasm_path).
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
    ensure!(
        !REFEREE_TO_REFERRER.has(ctx.storage, referee),
        "referee {referee} already has a referrer"
    );

    // Save the referee-to-referrer relation.
    REFEREE_TO_REFERRER.save(ctx.storage, referee, &referrer)?;

    // Initialize per-referee statistics for the referrer.
    REFERRER_TO_REFEREE_STATISTICS.save(ctx.storage, (referrer, referee), &RefereeStats {
        registered_at: ctx.block.timestamp,
        ..Default::default()
    })?;

    // Increment the referrer's referee count.
    {
        let today = round_to_day(ctx.block.timestamp);

        let mut data = load_referral_data(ctx.storage, referrer, None)?;
        data.referee_count += 1;

        USER_REFERRAL_DATA.save(ctx.storage, (referrer, today), &data)?;
    }

    Ok(Response::new().add_event(ReferralSet { referrer, referee })?)
}
