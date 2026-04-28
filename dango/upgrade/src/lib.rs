use {
    dango_perps::state::VAULT_SNAPSHOTS,
    dango_types::{UsdValue, perps::VaultSnapshot},
    grug::{Addr, BlockInfo, StdResult, Storage, Timestamp, Uint128, addr},
    grug_app::{AppResult, CHAIN_ID, CONTRACT_NAMESPACE, StorageProvider},
};

const MAINNET_CHAIN_ID: &str = "dango-1";
const MAINNET_PERPS_ADDRESS: Addr = addr!("90bc84df68d1aa59a857e04ed529e9a26edbea4f");

const TESTNET_CHAIN_ID: &str = "dango-testnet-1";
const TESTNET_PERPS_ADDRESS: Addr = addr!("f6344c5e2792e8f9202c58a2d88fbbde4cd3142f");

/// Daily noon-UTC snapshots of `(timestamp_ms, equity_raw_u128, share_supply_u128)`
/// for the market-making vault, pulled from the indexer. Covers protocol
/// launch (Apr 9, 2026) through Apr 28, 2026.
///
/// Apr 13 and Apr 14 carry forward Apr 12's values: the chain was halted on
/// those two days due to a security incident, so on-chain vault state did
/// not change.
///
/// Equity values are the raw 6-decimal fixed-point form: e.g.
/// `101_470_582_290` represents $101_470.582290.
const VAULT_SNAPSHOT_BACKFILL: &[(u128, i128, u128)] = &[
    (1_775_736_000_000, 101_470_582_290, 100_911_643_265),
    (1_775_822_400_000, 234_857_408_905, 221_237_371_759),
    (1_775_908_800_000, 499_173_687_995, 475_080_160_193),
    (1_775_995_200_000, 1_007_026_275_440, 951_591_253_818),
    // Apr 13 — chain halted; values carried forward from Apr 12.
    (1_776_081_600_000, 1_007_026_275_440, 951_591_253_818),
    // Apr 14 — chain halted; values carried forward from Apr 12.
    (1_776_168_000_000, 1_007_026_275_440, 951_591_253_818),
    (1_776_254_400_000, 1_000_880_038_081, 940_005_123_389),
    (1_776_340_800_000, 1_744_977_009_463, 1_614_965_273_200),
    (1_776_427_200_000, 1_926_781_167_149, 1_755_314_975_011),
    (1_776_513_600_000, 1_894_929_651_138, 1_719_929_571_778),
    (1_776_600_000_000, 1_890_817_127_571, 1_718_648_755_135),
    (1_776_686_400_000, 1_510_057_853_266, 1_374_879_845_175),
    (1_776_772_800_000, 1_667_591_153_086, 1_476_350_771_486),
    (1_776_859_200_000, 1_896_622_916_169, 1_636_508_129_302),
    (1_776_945_600_000, 2_034_941_574_815, 1_747_008_495_896),
    (1_777_032_000_000, 2_387_017_475_313, 2_035_857_630_119),
    (1_777_118_400_000, 2_413_036_742_261, 2_059_773_218_897),
    (1_777_204_800_000, 2_412_398_372_203, 2_053_401_794_842),
    (1_777_291_200_000, 2_429_661_573_824, 2_070_384_602_696),
    (1_777_377_600_000, 2_545_034_915_578, 2_182_925_867_463),
];

pub fn do_upgrade<VM>(storage: Box<dyn Storage>, _vm: VM, _block: BlockInfo) -> AppResult<()> {
    // Find the address of the perps contract corresponding to the current chain.
    let chain_id = CHAIN_ID.load(&storage)?;
    let perps_address = match chain_id.as_str() {
        MAINNET_CHAIN_ID => MAINNET_PERPS_ADDRESS,
        TESTNET_CHAIN_ID => TESTNET_PERPS_ADDRESS,
        _ => panic!("unknown chain id: {chain_id}"),
    };

    // Create the prefixed storage for the perps contract.
    let mut perps_storage = StorageProvider::new(storage, &[CONTRACT_NAMESPACE, &perps_address]);

    // The CSV-sourced backfill values are mainnet-specific. Testnet has its
    // own vault state, so its snapshots accumulate forward only from the
    // upgrade onwards.
    if chain_id.as_str() == MAINNET_CHAIN_ID {
        do_vault_snapshot_backfill(&mut perps_storage)?;
    }

    Ok(())
}

/// Insert pre-computed daily noon-UTC vault snapshots into `VAULT_SNAPSHOTS`.
fn do_vault_snapshot_backfill(storage: &mut dyn Storage) -> StdResult<()> {
    for &(ts_ms, equity_raw, share_supply) in VAULT_SNAPSHOT_BACKFILL {
        let key = Timestamp::from_millis(ts_ms);
        let snapshot = VaultSnapshot {
            equity: UsdValue::new_raw(equity_raw),
            share_supply: Uint128::new(share_supply),
        };
        VAULT_SNAPSHOTS.save(storage, key, &snapshot)?;
    }

    tracing::info!(
        "Backfilled {} vault snapshots",
        VAULT_SNAPSHOT_BACKFILL.len()
    );

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        grug::{MockStorage, Order},
    };

    /// Backfill writes all 20 daily samples into `VAULT_SNAPSHOTS` at the
    /// expected noon-UTC keys, with Apr 13/14 carrying forward Apr 12.
    #[test]
    fn vault_snapshot_backfill() {
        let mut storage = MockStorage::new();

        do_vault_snapshot_backfill(&mut storage).unwrap();

        let entries: Vec<(Timestamp, VaultSnapshot)> = VAULT_SNAPSHOTS
            .range(&storage, None, None, Order::Ascending)
            .collect::<StdResult<_>>()
            .unwrap();

        // 20 entries: Apr 9 through Apr 28, 2026.
        assert_eq!(entries.len(), 20);

        // Keys are strictly ascending.
        for win in entries.windows(2) {
            assert!(win[0].0 < win[1].0);
        }

        // Apr 9 (first entry).
        assert_eq!(entries[0].0, Timestamp::from_millis(1_775_736_000_000));
        assert_eq!(entries[0].1.equity, UsdValue::new_raw(101_470_582_290));
        assert_eq!(entries[0].1.share_supply, Uint128::new(100_911_643_265));

        // Apr 28 (last entry).
        assert_eq!(entries[19].0, Timestamp::from_millis(1_777_377_600_000));
        assert_eq!(entries[19].1.equity, UsdValue::new_raw(2_545_034_915_578));
        assert_eq!(entries[19].1.share_supply, Uint128::new(2_182_925_867_463));

        // Apr 12, 13, 14 carry the same `(equity, share_supply)`.
        let apr_12 = VAULT_SNAPSHOTS
            .load(&storage, Timestamp::from_millis(1_775_995_200_000))
            .unwrap();
        let apr_13 = VAULT_SNAPSHOTS
            .load(&storage, Timestamp::from_millis(1_776_081_600_000))
            .unwrap();
        let apr_14 = VAULT_SNAPSHOTS
            .load(&storage, Timestamp::from_millis(1_776_168_000_000))
            .unwrap();
        assert_eq!(apr_12.equity, apr_13.equity);
        assert_eq!(apr_13.equity, apr_14.equity);
        assert_eq!(apr_12.share_supply, apr_13.share_supply);
        assert_eq!(apr_13.share_supply, apr_14.share_supply);
    }
}
