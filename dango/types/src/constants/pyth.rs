//! Pyth price IDs can be found at: <https://www.pyth.network/price-feeds>.

use {
    crate::{
        constants::{eth, perp_btc, perp_eth, perp_hype, perp_sol, usdc},
        oracle::{PriceConfig, PriceSource},
    },
    grug_types::{Denom, btree_map},
    pyth_types::constants::{BTC_USD_ID, ETH_USD_ID, HYPE_USD_ID, SOL_USD_ID, USDC_USD_ID},
    std::{collections::BTreeMap, sync::LazyLock},
};

pub static PYTH_PRICE_SOURCES: LazyLock<BTreeMap<Denom, PriceConfig>> = LazyLock::new(|| {
    btree_map! {
        // ---------- Spot ----------
        eth::DENOM.clone() => PriceConfig::single(PriceSource {
            id: ETH_USD_ID.id,
            channel: ETH_USD_ID.channel,
        }),
        usdc::DENOM.clone() => PriceConfig::single(PriceSource {
            id: USDC_USD_ID.id,
            channel: USDC_USD_ID.channel,
        }),
        // ---------- Perp ----------
        perp_btc::DENOM.clone() => PriceConfig::single(PriceSource {
            id: BTC_USD_ID.id,
            channel: BTC_USD_ID.channel,
        }),
        perp_eth::DENOM.clone() => PriceConfig::single(PriceSource {
            id: ETH_USD_ID.id,
            channel: ETH_USD_ID.channel,
        }),
        perp_hype::DENOM.clone() => PriceConfig::single(PriceSource {
            id: HYPE_USD_ID.id,
            channel: HYPE_USD_ID.channel,
        }),
        perp_sol::DENOM.clone() => PriceConfig::single(PriceSource {
            id: SOL_USD_ID.id,
            channel: SOL_USD_ID.channel,
        }),
    }
});
