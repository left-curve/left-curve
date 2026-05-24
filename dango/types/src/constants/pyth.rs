//! Pyth price IDs can be found at: <https://www.pyth.network/price-feeds>.

use {
    crate::{
        constants::{atom, bch, bnb, btc, doge, eth, ltc, perp_eth, sol, usdc, xrp},
        oracle::PriceSource,
    },
    grug_types::{Denom, btree_map},
    pyth_types::constants::{
        ATOM_USD_ID, BCH_USD_ID, BNB_USD_ID, BTC_USD_ID, DOGE_USD_ID, ETH_USD_ID, LTC_USD_ID,
        SOL_USD_ID, USDC_USD_ID, XRP_USD_ID,
    },
    std::{collections::BTreeMap, sync::LazyLock},
};

pub static PYTH_PRICE_SOURCES: LazyLock<BTreeMap<Denom, PriceSource>> = LazyLock::new(|| {
    btree_map! {
        atom::DENOM.clone() => PriceSource {
            id: ATOM_USD_ID.id,
            channel: ATOM_USD_ID.channel,
        },
        bch::DENOM.clone() => PriceSource {
            id: BCH_USD_ID.id,
            channel: BCH_USD_ID.channel,
        },
        bnb::DENOM.clone() => PriceSource {
            id: BNB_USD_ID.id,
            channel: BNB_USD_ID.channel,
        },
        btc::DENOM.clone() => PriceSource {
            id: BTC_USD_ID.id,
            channel: BTC_USD_ID.channel,
        },
        doge::DENOM.clone() => PriceSource {
            id: DOGE_USD_ID.id,
            channel: DOGE_USD_ID.channel,
        },
        eth::DENOM.clone() => PriceSource {
            id: ETH_USD_ID.id,
            channel: ETH_USD_ID.channel,
        },
        ltc::DENOM.clone() => PriceSource {
            id: LTC_USD_ID.id,
            channel: LTC_USD_ID.channel,
        },
        sol::DENOM.clone() => PriceSource {
            id: SOL_USD_ID.id,
            channel: SOL_USD_ID.channel,
        },
        usdc::DENOM.clone() => PriceSource {
            id: USDC_USD_ID.id,
            channel: USDC_USD_ID.channel,
        },
        xrp::DENOM.clone() => PriceSource {
            id: XRP_USD_ID.id,
            channel: XRP_USD_ID.channel,
        },
        perp_eth::DENOM.clone() => PriceSource {
            id: ETH_USD_ID.id,
            channel: ETH_USD_ID.channel,
        },
    }
});
