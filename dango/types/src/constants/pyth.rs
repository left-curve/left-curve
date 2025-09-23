//! Pyth price IDs can be found at: <https://www.pyth.network/price-feeds>.

use {
    crate::{
        constants::{atom, bch, bnb, btc, doge, eth, ltc, sol, usdc, xrp},
        oracle::PriceSource,
    },
    grug::{Denom, btree_map},
    pyth_types::constants::{
        ATOM_USD_ID_LAZER, BCH_USD_ID_LAZER, BNB_USD_ID_LAZER, BTC_USD_ID_LAZER, DOGE_USD_ID_LAZER,
        ETH_USD_ID_LAZER, LTC_USD_ID_LAZER, SOL_USD_ID_LAZER, USDC_USD_ID_LAZER, XRP_USD_ID_LAZER,
    },
    std::{collections::BTreeMap, sync::LazyLock},
};

pub static PYTH_PRICE_SOURCES: LazyLock<BTreeMap<Denom, PriceSource>> = LazyLock::new(|| {
    btree_map! {
        atom::DENOM.clone() => PriceSource::Pyth {
            id: ATOM_USD_ID_LAZER.id,
            channel: ATOM_USD_ID_LAZER.channel,
            precision: 6,
        },
        bch::DENOM.clone() => PriceSource::Pyth {
            id: BCH_USD_ID_LAZER.id,
            channel: BCH_USD_ID_LAZER.channel,
            precision: 8,
        },
        bnb::DENOM.clone() => PriceSource::Pyth {
            id: BNB_USD_ID_LAZER.id,
            channel: BNB_USD_ID_LAZER.channel,
            precision: 18,
        },
        btc::DENOM.clone() => PriceSource::Pyth {
            id: BTC_USD_ID_LAZER.id,
            channel: BTC_USD_ID_LAZER.channel,
            precision: 8,
        },
        doge::DENOM.clone() => PriceSource::Pyth {
            id: DOGE_USD_ID_LAZER.id,
            channel: DOGE_USD_ID_LAZER.channel,
            precision: 8,
        },
        eth::DENOM.clone() => PriceSource::Pyth {
            id: ETH_USD_ID_LAZER.id,
            channel: ETH_USD_ID_LAZER.channel,
            precision: 18,
        },
        ltc::DENOM.clone() => PriceSource::Pyth {
            id: LTC_USD_ID_LAZER.id,
            channel: LTC_USD_ID_LAZER.channel,
            precision: 8,
        },
        sol::DENOM.clone() => PriceSource::Pyth {
            id: SOL_USD_ID_LAZER.id,
            channel: SOL_USD_ID_LAZER.channel,
            precision: 9,
        },
        usdc::DENOM.clone() => PriceSource::Pyth {
            id: USDC_USD_ID_LAZER.id,
            channel: USDC_USD_ID_LAZER.channel,
            precision: 6,
        },
        xrp::DENOM.clone() => PriceSource::Pyth {
            id: XRP_USD_ID_LAZER.id,
            channel: XRP_USD_ID_LAZER.channel,
            precision: 6,
        },
    }
});
