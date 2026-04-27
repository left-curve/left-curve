use {
    crate::WITHDRAW_VOLUMES,
    grug::{
        Bound, Denom, Duration, IsZero, Number, NumberConst, Order as IterationOrder, StdResult,
        Storage, Timestamp, Uint128,
    },
};

const NANOS_PER_MINUTE: u128 = 60 * 1_000_000_000;

/// Trailing window over which outbound withdraws accumulate against the cap.
pub const ROLLING_WINDOW: Duration = Duration::from_hours(24);

/// Cron-time horizon beyond which `WITHDRAW_VOLUMES` entries are pruned.
/// Set to twice the rolling window so that, between cron ticks, baseline
/// lookups at `now − 24h` always have data to read.
pub const PRUNE_HORIZON: Duration = Duration::from_hours(48);

/// Round a timestamp down to the start of its minute bucket.
pub fn round_to_minute(ts: Timestamp) -> Timestamp {
    let nanos = ts.into_nanos();
    Timestamp::from_nanos(nanos - (nanos % NANOS_PER_MINUTE))
}

/// Sum of withdraws in the trailing 24h, computed as
/// `latest_cumulative − cumulative_at_or_before(now − 24h)`.
pub fn rolling_window_sum(
    storage: &dyn Storage,
    denom: &Denom,
    now: Timestamp,
) -> StdResult<Uint128> {
    let latest = WITHDRAW_VOLUMES
        .prefix(denom)
        .range(storage, None, None, IterationOrder::Descending)
        .next()
        .transpose()?
        .map(|(_, v)| v)
        .unwrap_or(Uint128::ZERO);

    let baseline_ts = now.saturating_sub(ROLLING_WINDOW);
    let baseline = WITHDRAW_VOLUMES
        .prefix(denom)
        .range(
            storage,
            None,
            Some(Bound::Inclusive(baseline_ts)),
            IterationOrder::Descending,
        )
        .next()
        .transpose()?
        .map(|(_, v)| v)
        .unwrap_or(Uint128::ZERO);

    Ok(latest.checked_sub(baseline)?)
}

/// Add `amount` to the cumulative for the bucket at `round_to_minute(now)`.
/// Stored as `latest_cumulative + amount`, mirroring the perps volume pattern.
pub fn record_withdraw(
    storage: &mut dyn Storage,
    denom: &Denom,
    now: Timestamp,
    amount: Uint128,
) -> StdResult<()> {
    if amount.is_zero() {
        return Ok(());
    }

    let bucket = round_to_minute(now);

    let latest = match WITHDRAW_VOLUMES.may_load(storage, (denom, bucket))? {
        Some(v) => v,
        None => WITHDRAW_VOLUMES
            .prefix(denom)
            .range(
                storage,
                None,
                Some(Bound::Exclusive(bucket)),
                IterationOrder::Descending,
            )
            .next()
            .transpose()?
            .map(|(_, v)| v)
            .unwrap_or(Uint128::ZERO),
    };

    WITHDRAW_VOLUMES.save(storage, (denom, bucket), &latest.checked_add(amount)?)
}

/// Drop entries for `denom` strictly older than `now − 48h`. Keeps enough
/// history to serve baseline lookups for the next 24h of withdraws.
pub fn prune_old_volumes(
    storage: &mut dyn Storage,
    denom: &Denom,
    now: Timestamp,
) -> StdResult<()> {
    let cutoff = now.saturating_sub(PRUNE_HORIZON);

    let stale: Vec<Timestamp> = WITHDRAW_VOLUMES
        .prefix(denom)
        .range(
            storage,
            None,
            Some(Bound::Exclusive(cutoff)),
            IterationOrder::Ascending,
        )
        .map(|res| res.map(|(ts, _)| ts))
        .collect::<StdResult<_>>()?;

    for ts in stale {
        WITHDRAW_VOLUMES.remove(storage, (denom, ts));
    }

    Ok(())
}

/// Drop every entry for `denom`. Used when admin removes a denom from the
/// rate-limit map so re-adding it later starts with a clean rolling window.
pub fn clear_volumes(storage: &mut dyn Storage, denom: &Denom) -> StdResult<()> {
    let keys: Vec<Timestamp> = WITHDRAW_VOLUMES
        .prefix(denom)
        .range(storage, None, None, IterationOrder::Ascending)
        .map(|res| res.map(|(ts, _)| ts))
        .collect::<StdResult<_>>()?;

    for ts in keys {
        WITHDRAW_VOLUMES.remove(storage, (denom, ts));
    }

    Ok(())
}
