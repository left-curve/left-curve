use {
    crate::state::FEE_RATE_OVERRIDES,
    anyhow::ensure,
    dango_order_book::Dimensionless,
    dango_types::perps::{Param, RateSchedule},
    grug::{Addr, Order as IterationOrder, Storage},
};

/// The pending fee-rate-override edit being validated, if any.
///
/// - `Insert(user, maker, taker)` — a new or updated override for `user`.
/// - `Delete(user)` — removing `user`'s stored override (falls back to the
///   tier schedule for that user).
#[derive(Debug, Clone, Copy)]
pub enum OverrideDelta {
    Insert(Addr, Dimensionless, Dimensionless),
    Delete(Addr),
}

impl OverrideDelta {
    fn user(&self) -> Addr {
        match self {
            OverrideDelta::Insert(u, ..) | OverrideDelta::Delete(u) => *u,
        }
    }
}

/// Enforce the net-fee sign invariant:
///
/// ```text
/// min_effective_taker_rate + min_effective_maker_rate ≥ 0
/// ```
///
/// where the minima range over (a) the tier-schedule base, (b) every tier
/// rate in the schedule, and (c) every stored fee-rate override — with
/// the supplied `pending` edit applied on top of (c).
///
/// This is the invariant that makes `net_fee = taker_fee + maker_fee ≥ 0`
/// hold on every possible fill, which in turn is what the per-fill
/// net-fee distribution in `settle_pnls` relies on for correct
/// commission allocation.
pub fn check_fee_sign_invariant(
    storage: &dyn Storage,
    param: &Param,
    pending: Option<OverrideDelta>,
) -> anyhow::Result<()> {
    let mut min_maker = schedule_min(&param.maker_fee_rates);
    let mut min_taker = schedule_min(&param.taker_fee_rates);

    // Fold stored overrides, suppressing the one being edited.
    let pending_user = pending.map(|p| p.user());
    for item in FEE_RATE_OVERRIDES.range(storage, None, None, IterationOrder::Ascending) {
        let (user, (maker, taker)) = item?;
        if pending_user == Some(user) {
            continue;
        }
        min_maker = min_maker.min(maker);
        min_taker = min_taker.min(taker);
    }

    // Apply the pending insert's own rates.
    if let Some(OverrideDelta::Insert(_, maker, taker)) = pending {
        min_maker = min_maker.min(maker);
        min_taker = min_taker.min(taker);
    }

    ensure!(
        min_taker.checked_add(min_maker)? >= Dimensionless::ZERO,
        "invalid fee rates: min effective taker rate ({min_taker}) + min effective maker rate ({min_maker}) < 0 — would permit a fill with negative net fee",
    );

    Ok(())
}

/// Return the lowest rate in a schedule (base + all tier rates).
fn schedule_min(schedule: &RateSchedule) -> Dimensionless {
    schedule
        .tiers
        .values()
        .copied()
        .fold(schedule.base, Dimensionless::min)
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_order_book::UsdValue,
        grug::{MockStorage, btree_map},
        std::collections::BTreeMap,
    };

    fn make_param(
        taker_base: Dimensionless,
        maker_base: Dimensionless,
        taker_tiers: BTreeMap<UsdValue, Dimensionless>,
        maker_tiers: BTreeMap<UsdValue, Dimensionless>,
    ) -> Param {
        Param {
            taker_fee_rates: RateSchedule {
                base: taker_base,
                tiers: taker_tiers,
            },
            maker_fee_rates: RateSchedule {
                base: maker_base,
                tiers: maker_tiers,
            },
            ..Default::default()
        }
    }

    #[test]
    fn schedule_only_satisfies_invariant() {
        let storage = MockStorage::new();
        let param = make_param(
            Dimensionless::new_raw(100),  // +1 bp taker base
            Dimensionless::new_raw(-100), // -1 bp maker base
            Default::default(),
            Default::default(),
        );
        check_fee_sign_invariant(&storage, &param, None).unwrap();
    }

    #[test]
    fn schedule_violates_invariant_rejected() {
        let storage = MockStorage::new();
        let param = make_param(
            Dimensionless::new_raw(100),  // +1 bp taker
            Dimensionless::new_raw(-200), // -2 bp maker — net negative
            Default::default(),
            Default::default(),
        );
        let err = check_fee_sign_invariant(&storage, &param, None)
            .unwrap_err()
            .to_string();
        assert!(err.contains("negative net fee"), "{err}");
    }

    #[test]
    fn maker_tier_drives_min_below_zero() {
        let storage = MockStorage::new();
        let param = make_param(
            Dimensionless::new_raw(200),
            Dimensionless::ZERO,
            Default::default(),
            // At high volume the maker tier rebates -3 bps, which beats the
            // taker's base +2 bps → rejected.
            btree_map! {
                UsdValue::new_int(1_000_000) => Dimensionless::new_raw(-300),
            },
        );
        let err = check_fee_sign_invariant(&storage, &param, None)
            .unwrap_err()
            .to_string();
        assert!(err.contains("negative net fee"), "{err}");
    }

    #[test]
    fn pending_override_insert_violation_rejected() {
        let storage = MockStorage::new();
        let param = make_param(
            Dimensionless::new_raw(200),
            Dimensionless::ZERO,
            Default::default(),
            Default::default(),
        );
        let user = Addr::mock(42);
        // Pending override grants user a -3 bp maker rate, which breaks the
        // invariant against the +2 bp taker schedule minimum.
        let err = check_fee_sign_invariant(
            &storage,
            &param,
            Some(OverrideDelta::Insert(
                user,
                Dimensionless::new_raw(-300),
                Dimensionless::ZERO,
            )),
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("negative net fee"), "{err}");
    }

    #[test]
    fn pending_override_insert_valid_accepted() {
        let storage = MockStorage::new();
        let param = make_param(
            Dimensionless::new_raw(300),
            Dimensionless::ZERO,
            Default::default(),
            Default::default(),
        );
        let user = Addr::mock(42);
        // -2 bp maker vs +3 bp taker: net +1 bp. Accepted.
        check_fee_sign_invariant(
            &storage,
            &param,
            Some(OverrideDelta::Insert(
                user,
                Dimensionless::new_raw(-200),
                Dimensionless::new_raw(300),
            )),
        )
        .unwrap();
    }

    #[test]
    fn stored_override_considered_in_min() {
        let mut storage = MockStorage::new();
        // Pre-populate an override that pushes min maker rate to -3 bp.
        FEE_RATE_OVERRIDES
            .save(
                &mut storage,
                Addr::mock(7),
                &(Dimensionless::new_raw(-300), Dimensionless::new_raw(100)),
            )
            .unwrap();
        // Tier schedule alone would be fine (taker 2 bp, maker 0), but the
        // stored override's -3 bp maker breaks against the 1 bp taker
        // override it carries.
        let param = make_param(
            Dimensionless::new_raw(200),
            Dimensionless::ZERO,
            Default::default(),
            Default::default(),
        );
        let err = check_fee_sign_invariant(&storage, &param, None)
            .unwrap_err()
            .to_string();
        assert!(err.contains("negative net fee"), "{err}");
    }

    #[test]
    fn pending_delete_unblocks_invariant() {
        let mut storage = MockStorage::new();
        // A stored override below the tier schedule's taker base would
        // normally break the invariant...
        let user = Addr::mock(7);
        FEE_RATE_OVERRIDES
            .save(
                &mut storage,
                user,
                &(Dimensionless::new_raw(-300), Dimensionless::new_raw(100)),
            )
            .unwrap();
        let param = make_param(
            Dimensionless::new_raw(200),
            Dimensionless::ZERO,
            Default::default(),
            Default::default(),
        );
        // ... but deleting it restores compliance.
        check_fee_sign_invariant(&storage, &param, Some(OverrideDelta::Delete(user))).unwrap();
    }
}
