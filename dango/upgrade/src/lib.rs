use {
    dango_types::{account_factory::UserIndex, perps},
    grug::{Addr, BlockInfo, Order as IterationOrder, StdResult, Storage, Timestamp, addr},
    grug_app::{AppResult, CHAIN_ID, CONTRACT_NAMESPACE, StorageProvider},
    std::collections::BTreeMap,
};

const MAINNET_CHAIN_ID: &str = "dango-1";
const MAINNET_PERPS_ADDRESS: Addr = addr!("90bc84df68d1aa59a857e04ed529e9a26edbea4f");

const TESTNET_CHAIN_ID: &str = "dango-testnet-1";
const TESTNET_PERPS_ADDRESS: Addr = addr!("f6344c5e2792e8f9202c58a2d88fbbde4cd3142f");

mod legacy {
    use dango_types::UsdValue;

    use super::*;

    /// Pre-upgrade layout of `UserReferralData`. Lacks the
    /// `cumulative_global_active_referees` field and uses the old name
    /// `cumulative_active_referees` (same Borsh position).
    #[derive(borsh::BorshDeserialize, borsh::BorshSerialize)]
    pub struct UserReferralData {
        pub volume: UsdValue,
        pub commission_shared_by_referrer: UsdValue,
        pub referee_count: u32,
        pub referees_volume: UsdValue,
        pub commission_earned_from_referees: UsdValue,
        pub cumulative_active_referees: u32,
    }

    pub const USER_REFERRAL_DATA: grug::Map<(UserIndex, Timestamp), UserReferralData> =
        grug::Map::new("ref_data");
}

pub fn do_upgrade<VM>(storage: Box<dyn Storage>, _vm: VM, _block: BlockInfo) -> AppResult<()> {
    let chain_id = CHAIN_ID.load(&storage)?;

    let perps_address = match chain_id.as_str() {
        MAINNET_CHAIN_ID => MAINNET_PERPS_ADDRESS,
        TESTNET_CHAIN_ID => TESTNET_PERPS_ADDRESS,
        _ => panic!("unknown chain id: {chain_id}"),
    };

    let mut storage = StorageProvider::new(storage, &[CONTRACT_NAMESPACE, &perps_address]);

    do_referral_activated_referees_upgrade(&mut storage)?;

    Ok(())
}

/// Migrate `USER_REFERRAL_DATA` from the old Borsh layout (missing
/// `cumulative_global_active_referees`) to the new layout. Computes the
/// activated-referee count for each referrer by scanning
/// `REFERRER_TO_REFEREE_STATISTICS` for referees whose `last_day_active` is
/// non-zero (indicating at least one trade).
///
/// For each referrer we set `cumulative_global_active_referees` on every
/// existing bucket to the computed total. This is slightly imprecise for
/// historical buckets (we back-project the current total), but the field
/// is monotonically non-decreasing and there is no way to reconstruct
/// the exact per-day activation history retroactively.
fn do_referral_activated_referees_upgrade(storage: &mut dyn Storage) -> StdResult<()> {
    // 1. Compute activated referees per referrer from REFERRER_TO_REFEREE_STATISTICS.
    let mut activated_per_referrer: BTreeMap<UserIndex, u32> = BTreeMap::new();

    for res in dango_perps::state::REFERRER_TO_REFEREE_STATISTICS.range(
        storage,
        None,
        None,
        IterationOrder::Ascending,
    ) {
        let ((referrer, _referee), stats) = res?;
        if stats.last_day_active != Timestamp::ZERO {
            *activated_per_referrer.entry(referrer).or_default() += 1;
        }
    }

    // 2. Read all legacy USER_REFERRAL_DATA entries.
    let entries: Vec<_> = legacy::USER_REFERRAL_DATA
        .range(storage, None, None, IterationOrder::Ascending)
        .collect::<StdResult<_>>()?;

    let entry_count = entries.len();

    // 3. Rewrite each entry with the new layout.
    for ((user, ts), old) in entries {
        let activated = activated_per_referrer.get(&user).copied().unwrap_or(0);

        let new = perps::UserReferralData {
            volume: old.volume,
            commission_shared_by_referrer: old.commission_shared_by_referrer,
            referee_count: old.referee_count,
            referees_volume: old.referees_volume,
            commission_earned_from_referees: old.commission_earned_from_referees,
            cumulative_daily_active_referees: old.cumulative_active_referees,
            cumulative_global_active_referees: activated,
        };

        dango_perps::state::USER_REFERRAL_DATA.save(storage, (user, ts), &new)?;
    }

    tracing::info!(
        "Migrated {entry_count} UserReferralData entries (added cumulative_global_active_referees, {} referrers have activated referees)",
        activated_per_referrer.len()
    );

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        crate::{do_referral_activated_referees_upgrade, legacy},
        dango_types::{UsdValue, account_factory::UserIndex, perps},
        grug::{MockStorage, Timestamp},
    };

    /// `do_referral_activated_referees_upgrade` rewrites legacy
    /// `UserReferralData` entries, adding `cumulative_global_active_referees`
    /// computed from referee stats.
    #[test]
    fn referral_activated_referees_migration() {
        let mut storage = MockStorage::new();

        let referrer: UserIndex = 1;
        let referee_a: UserIndex = 2;
        let referee_b: UserIndex = 3;
        let referee_c: UserIndex = 4;

        let day1 = Timestamp::from_seconds(86_400);
        let day2 = Timestamp::from_seconds(86_400 * 2);

        // Referee A has traded (last_day_active != 0).
        dango_perps::state::REFERRER_TO_REFEREE_STATISTICS
            .save(&mut storage, (referrer, referee_a), &perps::RefereeStats {
                registered_at: day1,
                volume: UsdValue::new_int(1_000),
                commission_earned: UsdValue::new_int(10),
                last_day_active: day1,
            })
            .unwrap();

        // Referee B has traded.
        dango_perps::state::REFERRER_TO_REFEREE_STATISTICS
            .save(&mut storage, (referrer, referee_b), &perps::RefereeStats {
                registered_at: day1,
                volume: UsdValue::new_int(500),
                commission_earned: UsdValue::new_int(5),
                last_day_active: day2,
            })
            .unwrap();

        // Referee C has NOT traded (last_day_active == 0).
        dango_perps::state::REFERRER_TO_REFEREE_STATISTICS
            .save(&mut storage, (referrer, referee_c), &perps::RefereeStats {
                registered_at: day2,
                ..Default::default()
            })
            .unwrap();

        // Seed two legacy USER_REFERRAL_DATA buckets for the referrer.
        legacy::USER_REFERRAL_DATA
            .save(&mut storage, (referrer, day1), &legacy::UserReferralData {
                volume: UsdValue::ZERO,
                commission_shared_by_referrer: UsdValue::ZERO,
                referee_count: 2,
                referees_volume: UsdValue::new_int(1_000),
                commission_earned_from_referees: UsdValue::new_int(10),
                cumulative_active_referees: 1,
            })
            .unwrap();

        legacy::USER_REFERRAL_DATA
            .save(&mut storage, (referrer, day2), &legacy::UserReferralData {
                volume: UsdValue::ZERO,
                commission_shared_by_referrer: UsdValue::ZERO,
                referee_count: 3,
                referees_volume: UsdValue::new_int(1_500),
                commission_earned_from_referees: UsdValue::new_int(15),
                cumulative_active_referees: 3,
            })
            .unwrap();

        do_referral_activated_referees_upgrade(&mut storage).unwrap();

        // Both buckets should now have cumulative_global_active_referees = 2
        // (referee_a and referee_b traded, referee_c did not).
        let migrated_day1 = dango_perps::state::USER_REFERRAL_DATA
            .load(&storage, (referrer, day1))
            .unwrap();
        assert_eq!(migrated_day1.cumulative_global_active_referees, 2);
        assert_eq!(migrated_day1.cumulative_daily_active_referees, 1);
        assert_eq!(migrated_day1.referee_count, 2);

        let migrated_day2 = dango_perps::state::USER_REFERRAL_DATA
            .load(&storage, (referrer, day2))
            .unwrap();
        assert_eq!(migrated_day2.cumulative_global_active_referees, 2);
        assert_eq!(migrated_day2.cumulative_daily_active_referees, 3);
        assert_eq!(migrated_day2.referee_count, 3);
    }

    /// A referrer with no referees who have traded gets
    /// `cumulative_global_active_referees = 0` after the migration.
    #[test]
    fn referral_activated_referees_migration_no_active() {
        let mut storage = MockStorage::new();

        let referrer: UserIndex = 1;
        let day1 = Timestamp::from_seconds(86_400);

        legacy::USER_REFERRAL_DATA
            .save(&mut storage, (referrer, day1), &legacy::UserReferralData {
                volume: UsdValue::ZERO,
                commission_shared_by_referrer: UsdValue::ZERO,
                referee_count: 2,
                referees_volume: UsdValue::ZERO,
                commission_earned_from_referees: UsdValue::ZERO,
                cumulative_active_referees: 0,
            })
            .unwrap();

        do_referral_activated_referees_upgrade(&mut storage).unwrap();

        let migrated = dango_perps::state::USER_REFERRAL_DATA
            .load(&storage, (referrer, day1))
            .unwrap();
        assert_eq!(migrated.cumulative_global_active_referees, 0);
        assert_eq!(migrated.cumulative_daily_active_referees, 0);
    }

    /// Multiple referrers are migrated independently — each gets their own
    /// `cumulative_global_active_referees` count based on their own referees.
    #[test]
    fn referral_activated_referees_migration_multiple_referrers() {
        let mut storage = MockStorage::new();

        let referrer_a: UserIndex = 1;
        let referrer_b: UserIndex = 5;
        let referee_1: UserIndex = 2;
        let referee_2: UserIndex = 3;
        let referee_3: UserIndex = 6;

        let day1 = Timestamp::from_seconds(86_400);

        // Referrer A: referee_1 has traded, referee_2 has not.
        dango_perps::state::REFERRER_TO_REFEREE_STATISTICS
            .save(
                &mut storage,
                (referrer_a, referee_1),
                &perps::RefereeStats {
                    registered_at: day1,
                    volume: UsdValue::new_int(100),
                    commission_earned: UsdValue::new_int(1),
                    last_day_active: day1,
                },
            )
            .unwrap();

        dango_perps::state::REFERRER_TO_REFEREE_STATISTICS
            .save(
                &mut storage,
                (referrer_a, referee_2),
                &perps::RefereeStats {
                    registered_at: day1,
                    ..Default::default()
                },
            )
            .unwrap();

        // Referrer B: referee_3 has traded.
        dango_perps::state::REFERRER_TO_REFEREE_STATISTICS
            .save(
                &mut storage,
                (referrer_b, referee_3),
                &perps::RefereeStats {
                    registered_at: day1,
                    volume: UsdValue::new_int(200),
                    commission_earned: UsdValue::new_int(2),
                    last_day_active: day1,
                },
            )
            .unwrap();

        // Legacy data for both referrers.
        legacy::USER_REFERRAL_DATA
            .save(
                &mut storage,
                (referrer_a, day1),
                &legacy::UserReferralData {
                    volume: UsdValue::ZERO,
                    commission_shared_by_referrer: UsdValue::ZERO,
                    referee_count: 2,
                    referees_volume: UsdValue::new_int(100),
                    commission_earned_from_referees: UsdValue::new_int(1),
                    cumulative_active_referees: 1,
                },
            )
            .unwrap();

        legacy::USER_REFERRAL_DATA
            .save(
                &mut storage,
                (referrer_b, day1),
                &legacy::UserReferralData {
                    volume: UsdValue::ZERO,
                    commission_shared_by_referrer: UsdValue::ZERO,
                    referee_count: 1,
                    referees_volume: UsdValue::new_int(200),
                    commission_earned_from_referees: UsdValue::new_int(2),
                    cumulative_active_referees: 1,
                },
            )
            .unwrap();

        do_referral_activated_referees_upgrade(&mut storage).unwrap();

        // Referrer A: 1 out of 2 referees traded.
        let migrated_a = dango_perps::state::USER_REFERRAL_DATA
            .load(&storage, (referrer_a, day1))
            .unwrap();
        assert_eq!(migrated_a.cumulative_global_active_referees, 1);
        assert_eq!(migrated_a.cumulative_daily_active_referees, 1);
        assert_eq!(migrated_a.referee_count, 2);

        // Referrer B: 1 out of 1 referees traded.
        let migrated_b = dango_perps::state::USER_REFERRAL_DATA
            .load(&storage, (referrer_b, day1))
            .unwrap();
        assert_eq!(migrated_b.cumulative_global_active_referees, 1);
        assert_eq!(migrated_b.cumulative_daily_active_referees, 1);
        assert_eq!(migrated_b.referee_count, 1);
    }
}
