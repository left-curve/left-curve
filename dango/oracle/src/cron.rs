use {
    crate::PRICE_SOURCES,
    dango_types::oracle::PriceConfig,
    grug_types::{Order, Response, StdResult, Storage, SudoCtx, Timestamp},
};

/// Advance any in-progress futures rolls whose final fixing has passed.
///
/// Triggered once per cron interval (see the chain's `cronjobs` config). For
/// each `Roll` config whose current roll has completed and that has a queued
/// successor, `next` is promoted to `current` and the following contract is
/// pulled from `upcoming`. The loop catches up across several completed rolls if
/// the cron has fallen behind. Most ticks are a no-op (no roll completed), and
/// single-source configs are skipped.
///
/// Note this only sets up the *next* roll; the blend weight itself is derived
/// from the block timestamp at read time, so a late cron tick never produces a
/// wrong price — the read clamps to 100% `next` until the advance happens.
pub fn cron_execute(ctx: SudoCtx) -> anyhow::Result<Response> {
    advance_rolls(ctx.storage, ctx.block.timestamp)?;

    Ok(Response::new())
}

fn advance_rolls(storage: &mut dyn Storage, now: Timestamp) -> anyhow::Result<()> {
    // Collect every config first (propagating a genuine decode error), so the
    // read borrow ends before we start writing.
    let configs = PRICE_SOURCES
        .range(storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    for (denom, config) in configs {
        let PriceConfig::Roll(mut roll) = config else {
            continue;
        };

        if !roll.should_advance(now) {
            continue;
        }

        // Catch up across every roll whose window has already passed.
        while roll.should_advance(now) {
            roll.advance()?;
        }

        PRICE_SOURCES.save(storage, &denom, &PriceConfig::Roll(roll))?;
    }

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_order_book::Dimensionless,
        dango_types::oracle::{Fixing, PriceSource, RollState, ScheduledRoll},
        grug_types::{Denom, MockStorage},
        pyth_types::Channel,
        std::{collections::VecDeque, str::FromStr},
    };

    fn src(id: u32) -> PriceSource {
        PriceSource {
            id,
            channel: Channel::RealTime,
        }
    }

    fn fixing(secs: u128, pct: i128) -> Fixing {
        Fixing {
            at: Timestamp::from_seconds(secs),
            next_weight: Dimensionless::new_percent(pct),
        }
    }

    #[test]
    fn cron_advances_only_completed_rolls() {
        let mut storage = MockStorage::default();
        let denom = Denom::from_str("perp/oil").unwrap();

        // Current roll ends at t=100; a successor (contract 3) is queued at t=1000.
        let roll = PriceConfig::Roll(RollState {
            current: src(1),
            next: src(2),
            fixings: vec![fixing(100, 100)],
            upcoming: VecDeque::from([ScheduledRoll {
                contract: src(3),
                fixings: vec![fixing(1_000, 100)],
            }]),
        });
        PRICE_SOURCES.save(&mut storage, &denom, &roll).unwrap();

        // Before the current roll completes: no change.
        advance_rolls(&mut storage, Timestamp::from_seconds(50)).unwrap();
        assert_eq!(PRICE_SOURCES.load(&storage, &denom).unwrap(), roll);

        // After it completes: `next` becomes `current`, the queued contract
        // becomes `next`, and the queue drains.
        advance_rolls(&mut storage, Timestamp::from_seconds(200)).unwrap();
        let PriceConfig::Roll(advanced) = PRICE_SOURCES.load(&storage, &denom).unwrap() else {
            panic!("expected a roll");
        };
        assert_eq!(advanced.current, src(2));
        assert_eq!(advanced.next, src(3));
        assert_eq!(advanced.fixings, vec![fixing(1_000, 100)]);
        assert!(advanced.upcoming.is_empty());
    }

    #[test]
    fn cron_catches_up_across_multiple_completed_rolls() {
        let mut storage = MockStorage::default();
        let denom = Denom::from_str("perp/oil").unwrap();

        // Two queued rolls, both already in the past at t=10_000.
        let roll = PriceConfig::Roll(RollState {
            current: src(1),
            next: src(2),
            fixings: vec![fixing(100, 100)],
            upcoming: VecDeque::from([
                ScheduledRoll {
                    contract: src(3),
                    fixings: vec![fixing(200, 100)],
                },
                ScheduledRoll {
                    contract: src(4),
                    fixings: vec![fixing(300, 100)],
                },
            ]),
        });
        PRICE_SOURCES.save(&mut storage, &denom, &roll).unwrap();

        advance_rolls(&mut storage, Timestamp::from_seconds(10_000)).unwrap();

        let PriceConfig::Roll(advanced) = PRICE_SOURCES.load(&storage, &denom).unwrap() else {
            panic!("expected a roll");
        };
        // Both rolls consumed: current=3, next=4, queue empty.
        assert_eq!(advanced.current, src(3));
        assert_eq!(advanced.next, src(4));
        assert!(advanced.upcoming.is_empty());
    }

    #[test]
    fn cron_ignores_single_source_configs() {
        let mut storage = MockStorage::default();
        let denom = Denom::from_str("eth").unwrap();
        let config = PriceConfig::single(src(1));
        PRICE_SOURCES.save(&mut storage, &denom, &config).unwrap();

        advance_rolls(&mut storage, Timestamp::from_seconds(10_000)).unwrap();

        assert_eq!(PRICE_SOURCES.load(&storage, &denom).unwrap(), config);
    }
}
