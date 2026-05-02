use {
    crate::{UsdValue, VOLUMES},
    grug::{Addr, Bound, Order as IterationOrder, StdResult, Storage, Timestamp},
    std::collections::BTreeMap,
};

pub const NANOS_PER_DAY: u128 = 24 * 60 * 60 * 1_000_000_000;

pub fn round_to_day(ts: Timestamp) -> Timestamp {
    let nanos = ts.into_nanos();
    Timestamp::from_nanos(nanos - (nanos % NANOS_PER_DAY))
}

/// Flush accumulated per-user notional volumes to the cumulative VOLUMES map.
pub fn flush_volumes(
    storage: &mut dyn Storage,
    block_time: Timestamp,
    volumes: &BTreeMap<Addr, UsdValue>,
) -> StdResult<()> {
    let today = round_to_day(block_time);

    for (&user, &notional) in volumes {
        if notional.is_zero() {
            continue;
        }

        let current = match VOLUMES.may_load(storage, (user, today))? {
            Some(v) => v,
            None => VOLUMES
                .prefix(user)
                .range(
                    storage,
                    None,
                    Some(Bound::Exclusive(today)),
                    IterationOrder::Descending,
                )
                .next()
                .transpose()?
                .map(|(_, v)| v)
                .unwrap_or(UsdValue::ZERO),
        };

        VOLUMES.save(storage, (user, today), &current.checked_add(notional)?)?;
    }

    Ok(())
}
