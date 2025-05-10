//! Pyth price IDs can be found at: <https://www.pyth.network/price-feeds>.

use {
    crate::{
        constants::{atom, bch, bnb, btc, doge, eth, ltc, sol, usdc, xrp},
        oracle::PriceSource,
    },
    grug::{Denom, btree_map},
    pyth_types::constants::{
        ATOM_USD_ID, BCH_USD_ID, BNB_USD_ID, BTC_USD_ID, DOGE_USD_ID, ETH_USD_ID, LTC_USD_ID,
        SOL_USD_ID, USDC_USD_ID, XRP_USD_ID,
    },
    std::{collections::BTreeMap, sync::LazyLock},
};

pub static PYTH_PRICE_SOURCES: LazyLock<BTreeMap<Denom, PriceSource>> = LazyLock::new(|| {
    btree_map! {
        atom::DENOM.clone() => PriceSource::Pyth {
            id: ATOM_USD_ID,
            precision: 6,
        },
        bch::DENOM.clone() => PriceSource::Pyth {
            id: BCH_USD_ID,
            precision: 8,
        },
        bnb::DENOM.clone() => PriceSource::Pyth {
            id: BNB_USD_ID,
            precision: 18,
        },
        btc::DENOM.clone() => PriceSource::Pyth {
            id: BTC_USD_ID,
            precision: 8,
        },
        doge::DENOM.clone() => PriceSource::Pyth {
            id: DOGE_USD_ID,
            precision: 8,
        },
        eth::DENOM.clone() => PriceSource::Pyth {
            id: ETH_USD_ID,
            precision: 18,
        },
        ltc::DENOM.clone() => PriceSource::Pyth {
            id: LTC_USD_ID,
            precision: 8,
        },
        sol::DENOM.clone() => PriceSource::Pyth {
            id: SOL_USD_ID,
            precision: 9,
        },
        usdc::DENOM.clone() => PriceSource::Pyth {
            id: USDC_USD_ID,
            precision: 6,
        },
        xrp::DENOM.clone() => PriceSource::Pyth {
            id: XRP_USD_ID,
            precision: 6,
        },
    }
});
