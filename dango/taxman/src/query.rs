use {
    crate::{
        CONFIG, FEE_SHARE_RATIO, REFEREE_TO_REFERRER, REFERRER_TO_REFEREE_STATISTICS,
        USER_REFERRAL_DATA, VOLUMES_BY_USER,
    },
    dango_types::{
        account_factory::UserIndex,
        taxman::{
            CommissionRebund, Config, QueryMsg, Referee, RefereeStats, ReferralSettings, Referrer,
            ReferrerStatsOrderBy, ReferrerStatsOrderIndex, UserReferralData,
        },
    },
    grug::{
        BlockInfo, Borsh, Bound, DEFAULT_PAGE_LIMIT, Duration, ImmutableCtx, Json, JsonSerExt,
        MultiIndex, Number, NumberConst, Order, PrefixBound, PrimaryKey, StdResult, Storage,
        Timestamp, Udec128_6,
    },
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> anyhow::Result<Json> {
    match msg {
        QueryMsg::Config {} => {
            let res = query_config(ctx)?;
            res.to_json_value()
        },
        QueryMsg::VolumeByUser { user, since } => {
            let res = query_volume_by_user(ctx, user, since)?;
            res.to_json_value()
        },
        QueryMsg::Referrer { user } => {
            let res = query_referrer(ctx, user)?;
            res.to_json_value()
        },
        QueryMsg::ReferralData { user, since } => {
            let res = query_referral_data(ctx, user, since)?;
            res.to_json_value()
        },
        QueryMsg::ReferrerToRefereeStats {
            referrer,
            order_by: order,
        } => {
            let res = query_referrer_to_referee_stats(ctx, referrer, order)?;
            res.to_json_value()
        },
        QueryMsg::ReferralSettings { user } => {
            let res = query_referral_settings(ctx, user)?;
            res.to_json_value()
        },
    }
    .map_err(Into::into)
}

fn query_config(ctx: ImmutableCtx) -> StdResult<Config> {
    CONFIG.load(ctx.storage)
}

fn query_volume_by_user(
    ctx: ImmutableCtx,
    user: UserIndex,
    since: Option<Timestamp>,
) -> anyhow::Result<Udec128_6> {
    let volume_now = VOLUMES_BY_USER
        .prefix(user)
        .values(ctx.storage, None, None, Order::Descending)
        .next()
        .transpose()?
        .unwrap_or(Udec128_6::ZERO);

    let volume_since = if let Some(since) = since {
        VOLUMES_BY_USER
            .prefix(user)
            .values(
                ctx.storage,
                None,
                Some(Bound::Inclusive(since)),
                Order::Descending,
            )
            .next()
            .transpose()?
            .unwrap_or(Udec128_6::ZERO)
    } else {
        Udec128_6::ZERO
    };

    Ok(volume_now.checked_sub(volume_since)?)
}

fn query_referrer(ctx: ImmutableCtx, user: Referee) -> StdResult<Option<UserIndex>> {
    REFEREE_TO_REFERRER.may_load(ctx.storage, user)
}

fn query_referral_data(
    ctx: ImmutableCtx,
    user: UserIndex,
    since: Option<Timestamp>,
) -> anyhow::Result<UserReferralData> {
    let Some(data_now) = USER_REFERRAL_DATA
        .prefix(user)
        .values(ctx.storage, None, None, Order::Descending)
        .next()
        .transpose()?
    else {
        return Ok(UserReferralData::default());
    };

    let data_since = if let Some(since) = since {
        USER_REFERRAL_DATA
            .prefix(user)
            .values(
                ctx.storage,
                None,
                Some(Bound::Inclusive(since)),
                Order::Descending,
            )
            .next()
            .transpose()?
            .unwrap_or_default()
    } else {
        UserReferralData::default()
    };

    Ok(data_now.checked_sub(&data_since)?)
}

fn query_referrer_to_referee_stats(
    ctx: ImmutableCtx,
    referrer: Referrer,
    order_by: ReferrerStatsOrderBy,
) -> StdResult<Vec<(Referee, RefereeStats)>> {
    let limit = order_by.limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    let data = match order_by.index {
        ReferrerStatsOrderIndex::Commission { start_after } => {
            let index = REFERRER_TO_REFEREE_STATISTICS.idx.commission;

            collect_referee_stats(
                ctx.storage,
                index,
                referrer,
                start_after,
                limit,
                order_by.order,
            )
        },
        ReferrerStatsOrderIndex::RegisterAt { start_after } => {
            let index = REFERRER_TO_REFEREE_STATISTICS.idx.register_at;

            collect_referee_stats(
                ctx.storage,
                index,
                referrer,
                start_after,
                limit,
                order_by.order,
            )
        },
        ReferrerStatsOrderIndex::Volume { start_after } => {
            let index = REFERRER_TO_REFEREE_STATISTICS.idx.volume;
            collect_referee_stats(
                ctx.storage,
                index,
                referrer,
                start_after,
                limit,
                order_by.order,
            )
        },
    }?;

    Ok(data)
}

fn collect_referee_stats<S>(
    storage: &dyn Storage,
    index: MultiIndex<(u32, u32), (u32, S), RefereeStats, Borsh>,
    referrer: Referrer,
    start_after: Option<S>,
    limit: usize,
    order: Order,
) -> StdResult<Vec<(u32, RefereeStats)>>
where
    S: PrimaryKey,
{
    let start_after = start_after.map(PrefixBound::Exclusive);

    let (min, max) = match order {
        Order::Ascending => (start_after, None),
        Order::Descending => (None, start_after),
    };

    index
        .sub_prefix(referrer)
        .prefix_range(storage, min, max, order)
        .take(limit)
        .map(|value| {
            let ((_, referee), referee_stats) = value?;
            Ok((referee, referee_stats))
        })
        .collect::<StdResult<Vec<_>>>()
}

fn query_referral_settings(
    ctx: ImmutableCtx,
    user: UserIndex,
) -> anyhow::Result<Option<ReferralSettings>> {
    referral_settings(ctx.storage, user, ctx.block)
}

/// Retrieve the most recent record of the user's cumulative data.
/// If none exists, return the default value.
pub(crate) fn last_user_referral_data(
    storage: &dyn Storage,
    user: UserIndex,
) -> anyhow::Result<UserReferralData> {
    let (_, data) = USER_REFERRAL_DATA
        .prefix(user)
        .range(storage, None, None, Order::Descending)
        .next()
        .transpose()?
        .unwrap_or_default();

    Ok(data)
}

/// Return the ReferralSettings for a specific user.
pub(crate) fn referral_settings(
    storage: &dyn Storage,
    user: UserIndex,
    block_info: BlockInfo,
) -> anyhow::Result<Option<ReferralSettings>> {
    if let Some(share_ratio) = FEE_SHARE_RATIO.may_load(storage, user)? {
        let commission_rebund = calculate_commission_rebund(storage, user, block_info)?;
        return Ok(Some(ReferralSettings {
            commission_rebund,
            share_ratio,
        }));
    }

    Ok(None)
}

/// Calculate the commission rebound ratio for a referrer.
fn calculate_commission_rebund(
    storage: &dyn Storage,
    referrer: Referrer,
    block_info: BlockInfo,
) -> anyhow::Result<CommissionRebund> {
    // Retrieve the last user data for the referrer.
    let data_last = last_user_referral_data(storage, referrer)?;

    let since = block_info
        .timestamp
        .saturating_sub(Duration::from_days(30))
        .truncate_to_days();

    // Retrieve the user data since 30 days ago.
    let data_since = USER_REFERRAL_DATA
        .prefix(referrer)
        .values(
            storage,
            None,
            Some(grug::Bound::Inclusive(since)),
            Order::Descending,
        )
        .next()
        .transpose()?
        .unwrap_or_default();

    // Calculate the volume the referees traded in the last 30 days.
    let referees_volume = data_last
        .referees_volume
        .checked_sub(data_since.referees_volume)?;

    // Determine the commission rebund ratio based on the referees volume.
    let referral_config = CONFIG.load(storage)?.referral;

    let mut referrer_commission_rebound = referral_config.commission_rebound_default;

    for (volume_threshold, commission_rebound) in referral_config.commission_rebound_by_volume {
        if referees_volume >= volume_threshold {
            referrer_commission_rebound = commission_rebound;
        } else {
            break;
        }
    }

    Ok(referrer_commission_rebound)
}
