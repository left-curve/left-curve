use {
    crate::{
        core::compute_user_equity,
        querier::NoCachePerpQuerier,
        state::{STATE, USER_STATES, VAULT_SNAPSHOTS},
        volume::round_to_day,
    },
    dango_oracle::OracleQuerier,
    dango_types::perps::VaultSnapshot,
    grug::{Addr, Storage, Timestamp},
};

/// Take a daily snapshot of the market-making vault's `(equity, share_supply)`.
///
/// Snapshots are keyed by `current_time` rounded down to the start of the day.
/// If a snapshot for today's bucket already exists, this is a no-op. If equity
/// computation fails (e.g. transient stale-oracle error), the snapshot is
/// skipped — a later block in the same day can fill the bucket. Snapshots
/// must never halt cron, so failures in equity computation are not propagated.
pub fn take_vault_snapshot(
    storage: &mut dyn Storage,
    current_time: Timestamp,
    contract: Addr,
    oracle_querier: &mut OracleQuerier,
) -> anyhow::Result<()> {
    let key = round_to_day(current_time);

    // Idempotent within a day: skip if an earlier block today already wrote
    // the snapshot.
    if VAULT_SNAPSHOTS.has(storage, key) {
        return Ok(());
    }

    let state = STATE.load(storage)?;
    let vault_state = USER_STATES.may_load(storage, contract)?.unwrap_or_default();
    let perp_querier = NoCachePerpQuerier::new_local(storage);

    let equity = match compute_user_equity(oracle_querier, &perp_querier, &vault_state) {
        Ok(e) => e,
        Err(_err) => {
            #[cfg(feature = "tracing")]
            tracing::warn!(
                error = %_err,
                "Failed to compute vault equity for snapshot; skipping"
            );
            return Ok(());
        },
    };

    VAULT_SNAPSHOTS.save(storage, key, &VaultSnapshot {
        equity,
        share_supply: state.vault_share_supply,
    })?;

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::state::PAIR_STATES,
        dango_types::{
            FundingPerUnit, Quantity, UsdPrice,
            oracle::PrecisionedPrice,
            perps::{PairId, PairState, Position, State, UserState},
        },
        grug::{MockStorage, Order, Udec128, Uint128, hash_map},
        std::collections::BTreeMap,
    };

    const CONTRACT: Addr = Addr::mock(100);
    const ONE_DAY: u128 = 86_400;

    fn btc_pair_id() -> PairId {
        "perp/btcusd".parse().unwrap()
    }

    /// Initialize the storage required by `take_vault_snapshot`: `STATE`, the
    /// vault `USER_STATE` (optionally with a BTC position), and `PAIR_STATES`
    /// for BTC so the equity-computation lookup succeeds.
    fn init_storage(storage: &mut dyn Storage, share_supply: u128, position_size: i128) {
        STATE
            .save(storage, &State {
                vault_share_supply: Uint128::new(share_supply),
                ..Default::default()
            })
            .unwrap();

        let mut positions = BTreeMap::new();
        if position_size != 0 {
            positions.insert(btc_pair_id(), Position {
                size: Quantity::new_int(position_size),
                entry_price: UsdPrice::ZERO,
                entry_funding_per_unit: FundingPerUnit::ZERO,
                conditional_order_above: None,
                conditional_order_below: None,
            });
        }

        USER_STATES
            .save(storage, CONTRACT, &UserState {
                positions,
                ..Default::default()
            })
            .unwrap();

        PAIR_STATES
            .save(storage, &btc_pair_id(), &PairState::default())
            .unwrap();
    }

    fn mock_oracle_for_btc() -> OracleQuerier<'static> {
        OracleQuerier::new_mock(hash_map! {
            btc_pair_id() => PrecisionedPrice::new(
                Udec128::new_percent(5_000_000), // $50,000
                Timestamp::from_seconds(0),
                8,
            ),
        })
    }

    #[test]
    fn snapshot_written_when_bucket_empty() {
        let mut storage = MockStorage::new();
        init_storage(&mut storage, 1_000_000, 0);
        let mut oracle = mock_oracle_for_btc();

        take_vault_snapshot(
            &mut storage,
            Timestamp::from_seconds(0),
            CONTRACT,
            &mut oracle,
        )
        .unwrap();

        let snapshot = VAULT_SNAPSHOTS
            .load(&storage, Timestamp::from_seconds(0))
            .unwrap();
        assert_eq!(snapshot.share_supply, Uint128::new(1_000_000));
    }

    #[test]
    fn snapshot_idempotent_within_day() {
        let mut storage = MockStorage::new();
        init_storage(&mut storage, 1_000_000, 0);
        let mut oracle = mock_oracle_for_btc();

        // First call records share_supply = 1_000_000.
        take_vault_snapshot(
            &mut storage,
            Timestamp::from_seconds(0),
            CONTRACT,
            &mut oracle,
        )
        .unwrap();

        // Mutate state so we can detect overwrites.
        let mut state = STATE.load(&storage).unwrap();
        state.vault_share_supply = Uint128::new(2_000_000);
        STATE.save(&mut storage, &state).unwrap();

        // Second call later the same day must NOT overwrite.
        take_vault_snapshot(
            &mut storage,
            Timestamp::from_seconds(ONE_DAY - 1),
            CONTRACT,
            &mut oracle,
        )
        .unwrap();

        let snapshot = VAULT_SNAPSHOTS
            .load(&storage, Timestamp::from_seconds(0))
            .unwrap();
        assert_eq!(snapshot.share_supply, Uint128::new(1_000_000));

        // Only one entry — no second-day key.
        let keys: Vec<_> = VAULT_SNAPSHOTS
            .keys(&storage, None, None, Order::Ascending)
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(keys, vec![Timestamp::from_seconds(0)]);
    }

    #[test]
    fn snapshot_written_for_each_new_day() {
        let mut storage = MockStorage::new();
        init_storage(&mut storage, 1_000_000, 0);
        let mut oracle = mock_oracle_for_btc();

        for day in 0u128..3 {
            take_vault_snapshot(
                &mut storage,
                Timestamp::from_seconds(day * ONE_DAY),
                CONTRACT,
                &mut oracle,
            )
            .unwrap();
        }

        let keys: Vec<_> = VAULT_SNAPSHOTS
            .keys(&storage, None, None, Order::Ascending)
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(keys, vec![
            Timestamp::from_seconds(0),
            Timestamp::from_seconds(ONE_DAY),
            Timestamp::from_seconds(2 * ONE_DAY),
        ]);
    }

    #[test]
    fn snapshot_skipped_on_equity_failure() {
        let mut storage = MockStorage::new();
        // Vault has a BTC position, but the oracle has no BTC price → equity
        // computation fails.
        init_storage(&mut storage, 1_000_000, 50);
        let mut oracle = OracleQuerier::new_mock(hash_map! {});

        // Must not propagate the error.
        take_vault_snapshot(
            &mut storage,
            Timestamp::from_seconds(0),
            CONTRACT,
            &mut oracle,
        )
        .unwrap();

        // No snapshot written.
        assert!(
            !VAULT_SNAPSHOTS.has(&storage, Timestamp::from_seconds(0)),
            "expected no snapshot when equity computation fails"
        );
    }

    #[test]
    fn snapshot_key_rounds_down_to_day() {
        let mut storage = MockStorage::new();
        init_storage(&mut storage, 1_000_000, 0);
        let mut oracle = mock_oracle_for_btc();

        // 1.5 days into the chain.
        let ts = Timestamp::from_seconds(ONE_DAY + ONE_DAY / 2);

        take_vault_snapshot(&mut storage, ts, CONTRACT, &mut oracle).unwrap();

        // Snapshot is keyed at the day boundary, not the raw timestamp.
        assert!(VAULT_SNAPSHOTS.has(&storage, Timestamp::from_seconds(ONE_DAY)));
        assert!(!VAULT_SNAPSHOTS.has(&storage, ts));
    }
}
