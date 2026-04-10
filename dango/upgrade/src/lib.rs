use {
    dango_types::{
        Dimensionless, FundingRate, Quantity, UsdPrice, UsdValue,
        perps::{self, PairId},
    },
    grug::{Addr, BlockInfo, Order as IterationOrder, StdResult, Storage, addr},
    grug_app::{AppResult, CHAIN_ID, CONTRACT_NAMESPACE, StorageProvider},
    std::collections::BTreeSet,
};

const MAINNET_CHAIN_ID: &str = "dango-1";
const MAINNET_PERPS_ADDRESS: Addr = addr!("90bc84df68d1aa59a857e04ed529e9a26edbea4f");

const TESTNET_CHAIN_ID: &str = "dango-testnet-1";
const TESTNET_PERPS_ADDRESS: Addr = addr!("f6344c5e2792e8f9202c58a2d88fbbde4cd3142f");

/// Legacy types matching the pre-upgrade Borsh layout.
///
/// `PairParam` before this upgrade does not contain the three inventory
/// skew fields: `vault_size_skew_factor`, `vault_spread_skew_factor`,
/// `vault_max_skew_size`.
mod legacy {
    use super::*;

    pub const PAIR_PARAMS: grug::Map<&PairId, PairParam> = grug::Map::new("pair_param");

    #[derive(borsh::BorshDeserialize, borsh::BorshSerialize)]
    pub struct PairParam {
        pub tick_size: UsdPrice,
        pub min_order_size: UsdValue,
        pub max_abs_oi: Quantity,
        pub max_abs_funding_rate: FundingRate,
        pub initial_margin_ratio: Dimensionless,
        pub maintenance_margin_ratio: Dimensionless,
        pub impact_size: UsdValue,
        pub vault_liquidity_weight: Dimensionless,
        pub vault_half_spread: Dimensionless,
        pub vault_max_quote_size: Quantity,
        pub bucket_sizes: BTreeSet<UsdPrice>,
    }
}

pub fn do_upgrade<VM>(storage: Box<dyn Storage>, _vm: VM, _block: BlockInfo) -> AppResult<()> {
    let chain_id = CHAIN_ID.load(&storage)?;

    let perps_address = match chain_id.as_str() {
        MAINNET_CHAIN_ID => MAINNET_PERPS_ADDRESS,
        TESTNET_CHAIN_ID => TESTNET_PERPS_ADDRESS,
        _ => panic!("unknown chain id: {chain_id}"),
    };

    let mut storage = StorageProvider::new(storage, &[CONTRACT_NAMESPACE, &perps_address]);

    Ok(_do_upgrade(&mut storage)?)
}

fn _do_upgrade(storage: &mut dyn Storage) -> StdResult<()> {
    let old_params: Vec<_> = legacy::PAIR_PARAMS
        .range(storage, None, None, IterationOrder::Ascending)
        .collect::<StdResult<_>>()?;

    let count = old_params.len();

    for (pair_id, old) in old_params {
        let new = perps::PairParam {
            tick_size: old.tick_size,
            min_order_size: old.min_order_size,
            max_abs_oi: old.max_abs_oi,
            max_abs_funding_rate: old.max_abs_funding_rate,
            initial_margin_ratio: old.initial_margin_ratio,
            maintenance_margin_ratio: old.maintenance_margin_ratio,
            impact_size: old.impact_size,
            vault_liquidity_weight: old.vault_liquidity_weight,
            vault_half_spread: old.vault_half_spread,
            vault_max_quote_size: old.vault_max_quote_size,
            // New fields — disabled (zero) until governance sets real values.
            vault_size_skew_factor: Dimensionless::ZERO,
            vault_spread_skew_factor: Dimensionless::ZERO,
            vault_max_skew_size: Quantity::ZERO,
            bucket_sizes: old.bucket_sizes,
        };

        dango_perps::state::PAIR_PARAMS.save(storage, &pair_id, &new)?;
    }

    tracing::info!("Migrated {count} PairParam entries (added inventory skew fields)");

    Ok(())
}
