//! Pyth price IDs can be found at: <https://www.pyth.network/price-feeds>.

use {
    crate::{
        constants::{atom, bch, bnb, btc, doge, eth, ltc, sol, usd, xrp},
        oracle::PriceSource,
    },
    grug::{Denom, Timestamp, Udec128, btree_map},
    pyth_types::constants::{
        ATOM_USD_ID, BCH_USD_ID, BNB_USD_ID, BTC_USD_ID, DOGE_USD_ID, ETH_USD_ID, LTC_USD_ID,
        SOL_USD_ID, XRP_USD_ID,
    },
    std::{collections::BTreeMap, sync::LazyLock},
};

pub static PYTH_PRICE_SOURCES: LazyLock<BTreeMap<Denom, PriceSource>> = LazyLock::new(|| {
    btree_map! {
        atom::DENOM.clone() => PriceSource::Pyth {
            id: ATOM_USD_ID.id,
            channel: ATOM_USD_ID.channel,
            precision: 6,
        },
        bch::DENOM.clone() => PriceSource::Pyth {
            id: BCH_USD_ID.id,
            channel: BCH_USD_ID.channel,
            precision: 8,
        },
        bnb::DENOM.clone() => PriceSource::Pyth {
            id: BNB_USD_ID.id,
            channel: BNB_USD_ID.channel,
            precision: 18,
        },
        btc::DENOM.clone() => PriceSource::Pyth {
            id: BTC_USD_ID.id,
            channel: BTC_USD_ID.channel,
            precision: 8,
        },
        doge::DENOM.clone() => PriceSource::Pyth {
            id: DOGE_USD_ID.id,
            channel: DOGE_USD_ID.channel,
            precision: 8,
        },
        eth::DENOM.clone() => PriceSource::Pyth {
            id: ETH_USD_ID.id,
            channel: ETH_USD_ID.channel,
            precision: 18,
        },
        ltc::DENOM.clone() => PriceSource::Pyth {
            id: LTC_USD_ID.id,
            channel: LTC_USD_ID.channel,
            precision: 8,
        },
        sol::DENOM.clone() => PriceSource::Pyth {
            id: SOL_USD_ID.id,
            channel: SOL_USD_ID.channel,
            precision: 9,
        },
        // Fix the price of USD to $1.
        usd::DENOM.clone() => PriceSource::Fixed {
            humanized_price: Udec128::new(1),
            precision: 6,
            timestamp: Timestamp::from_nanos(u128::MAX),
        },
        xrp::DENOM.clone() => PriceSource::Pyth {
            id: XRP_USD_ID.id,
            channel: XRP_USD_ID.channel,
            precision: 6,
        },
    }
});
