use {
    dango_types::constants::{btc, eth, sol, usdc},
    grug::{Addr, BlockInfo, Denom, Storage, Uint128, addr},
    grug_app::{AppResult, CONTRACT_NAMESPACE, StorageProvider},
    std::sync::LazyLock,
};

/// The old data format.
mod legacy_dex {
    use {
        dango_types::dex::{self, PassiveLiquidity, Price},
        grug::{Bounded, Denom, Map, NonZero, Udec128, Uint128, ZeroExclusiveOneExclusive},
        std::collections::BTreeSet,
    };

    pub const PAIRS: Map<(&Denom, &Denom), PairParams> = Map::new("pair");

    #[grug::derive(Borsh)]
    pub struct PairParams {
        pub lp_denom: Denom,
        pub pool_type: PassiveLiquidity,
        pub bucket_sizes: BTreeSet<NonZero<Price>>,
        pub swap_fee_rate: Bounded<Udec128, ZeroExclusiveOneExclusive>,
        pub min_order_size: Uint128,
    }

    impl PairParams {
        pub fn into_new_format(self, min_order_size_base: Uint128) -> dex::PairParams {
            dex::PairParams {
                lp_denom: self.lp_denom,
                pool_type: self.pool_type,
                bucket_sizes: self.bucket_sizes,
                swap_fee_rate: self.swap_fee_rate,
                min_order_size_base,
                min_order_size_quote: self.min_order_size,
            }
        }
    }
}

/// Address of the DEX contract.
const DEX: Addr = addr!("8dd37b7e12d36bbe1c00ce9f0c341bfe1712e73f");

// (base_denom, quote_denom) => min_order_size_base.
// We set the minimum order size in the base asset to ~5 USD.
static PARAMS: [(&LazyLock<Denom>, &LazyLock<Denom>, u128); 3] = [
    // 5 (USD) / 100,000 (USD per BTC) * 100,000,000 (sats per BTC) = 5,000 sats
    (&btc::DENOM, &usdc::DENOM, 5_000),
    // 5 (USD) / 4,000 (USD per ETH) * 1e+18 (wei per ETH) = 1.25e+15 wei
    (&eth::DENOM, &usdc::DENOM, 1_250_000_000_000_000),
    // 5 (USD) / 200 (USD per SOL) * 1e+9 (lamports per ETH) = 25,000,000 lamports
    (&sol::DENOM, &usdc::DENOM, 25_000_000),
];

pub fn do_upgrade<VM>(storage: Box<dyn Storage>, _vm: VM, _block: BlockInfo) -> AppResult<()> {
    tracing::info!("Migrating DEX pair parameters");

    // Get the storage of the DEX contract.
    let mut dex_storage = StorageProvider::new(storage, &[CONTRACT_NAMESPACE, &DEX]);

    for (base_denom, quote_denom, min_order_size_base) in PARAMS {
        tracing::info!(
            base_denom = base_denom.to_string(),
            quote_denom = quote_denom.to_string(),
            "Migrating pair"
        );

        // Load the params in the old format.
        let legacy_params = legacy_dex::PAIRS.load(&dex_storage, (base_denom, quote_denom))?;

        // Convert the data to the new format.
        let params = legacy_params.into_new_format(Uint128::new(min_order_size_base));

        // Save the new format.
        dango_dex::PAIRS.save(&mut dex_storage, (base_denom, quote_denom), &params)?;
    }

    tracing::info!("Completed migrating DEX pair parameters");

    Ok(())
}
