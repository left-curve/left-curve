//! Pyth price IDs can be found at: <https://www.pyth.network/price-feeds>.

use {
    crate::{
        constants::{
            ATOM_DENOM, BCH_DENOM, BNB_DENOM, BTC_DENOM, DOGE_DENOM, ETH_DENOM, LTC_DENOM,
            SHIB_DENOM, SOL_DENOM, SUI_DENOM, USDC_DENOM, WBTC_DENOM, XRP_DENOM,
        },
        oracle::PriceSource,
    },
    grug::{btree_map, Denom},
    pyth_types::{
        ATOM_USD_ID, BCH_USD_ID, BNB_USD_ID, BTC_USD_ID, DOGE_USD_ID, ETH_USD_ID, LTC_USD_ID,
        SHIB_USD_ID, SOL_USD_ID, SUI_USD_ID, USDC_USD_ID, WBTC_USD_ID, XRP_USD_ID,
    },
    std::{collections::BTreeMap, sync::LazyLock},
};

pub static PYTH_PRICE_SOURCES: LazyLock<BTreeMap<Denom, PriceSource>> = LazyLock::new(|| {
    btree_map! {
        ATOM_DENOM.clone() => PriceSource::Pyth {
            id: ATOM_USD_ID,
            precision: 6,
        },
        BCH_DENOM.clone()  => PriceSource::Pyth {
            id: BCH_USD_ID,
            precision: 8,
        },
        BNB_DENOM.clone()  => PriceSource::Pyth {
            id: BNB_USD_ID,
            precision: 18,
        },
        BTC_DENOM.clone()  => PriceSource::Pyth {
            id: BTC_USD_ID,
            precision: 8,
        },
        DOGE_DENOM.clone() => PriceSource::Pyth {
            id: DOGE_USD_ID,
            precision: 8,
        },
        ETH_DENOM.clone()  => PriceSource::Pyth {
            id: ETH_USD_ID,
            precision: 18,
        },
        LTC_DENOM.clone()  => PriceSource::Pyth {
            id: LTC_USD_ID,
            precision: 8,
        },
        SHIB_DENOM.clone() => PriceSource::Pyth {
            id: SHIB_USD_ID,
            precision: 18,
        },
        SOL_DENOM.clone()  => PriceSource::Pyth {
            id: SOL_USD_ID,
            precision: 9,
        },
        SUI_DENOM.clone()  => PriceSource::Pyth {
            id: SUI_USD_ID,
            precision: 9,
        },
        USDC_DENOM.clone() => PriceSource::Pyth {
            id: USDC_USD_ID,
            precision: 6,
        },
        WBTC_DENOM.clone() => PriceSource::Pyth {
            id: WBTC_USD_ID,
            precision: 8,
        },
        XRP_DENOM.clone()  => PriceSource::Pyth {
            id: XRP_USD_ID,
            precision: 6,
        },
    }
});
