use {
    crate::{PRICE_SOURCES, PYTH_PRICES},
    anyhow::anyhow,
    dango_types::oracle::{Price, PriceSource},
    grug_storage::StorageQuerier,
    grug_types::{Addr, Cache, Denom, QuerierWrapper, StdResult, Storage},
    pyth_types::PythId,
    std::collections::HashMap,
};

pub struct OracleQuerier<'a> {
    cache: Cache<'a, Denom, Price, anyhow::Error, PriceSource>,
}

impl<'a> OracleQuerier<'a> {
    /// Create a new `OracleQuerier` for in another contract, with caching.
    pub fn new_remote(address: Addr, querier: QuerierWrapper<'a>) -> Self {
        let ctx = OracleContext::Remote { address, querier };
        let no_cache_querier = OracleQuerierNoCache::new(ctx);

        Self {
            cache: Cache::new(move |denom, price_source| {
                no_cache_querier.query_price(denom, price_source)
            }),
        }
    }

    /// Create a new `OracleQuerier` that returns predefined prices in a hash map.
    /// For using in tests.
    pub fn new_mock(prices: HashMap<Denom, Price>) -> Self {
        Self {
            cache: Cache::new(move |denom, _| {
                prices.get(denom).cloned().ok_or_else(|| {
                    anyhow!("[mock]: price not provided to oracle querier for denom `{denom}`")
                })
            }),
        }
    }

    pub fn query_price(
        &mut self,
        denom: &Denom,
        price_source: Option<PriceSource>,
    ) -> anyhow::Result<Price> {
        self.cache.get_or_fetch(denom, price_source).cloned()
    }
}

pub(crate) struct OracleQuerierNoCache<'a> {
    ctx: OracleContext<'a>,
}

impl<'a> OracleQuerierNoCache<'a> {
    /// Create a new `OracleQuerierNoCache` for use inside the oracle contract
    /// itself.
    pub fn new_local(storage: &'a dyn Storage) -> Self {
        Self::new(OracleContext::Local { storage })
    }

    fn new(ctx: OracleContext<'a>) -> Self {
        Self { ctx }
    }

    pub fn query_price(
        &self,
        denom: &Denom,
        price_source: Option<PriceSource>,
    ) -> anyhow::Result<Price> {
        // Query the denom's price source, if not provided.
        let price_source = price_source.map_or_else(|| self.ctx.get_price_source(denom), Ok)?;

        Ok(self.ctx.get_price(price_source.id)?)
    }
}

enum OracleContext<'a> {
    /// Used when oracle contract is the current contract.
    Local { storage: &'a dyn Storage },
    /// Used when oracle contract is another contract.
    Remote {
        address: Addr,
        querier: QuerierWrapper<'a>,
    },
}

#[rustfmt::skip]
impl OracleContext<'_> {
    fn get_price(&self, pyth_id: PythId) -> StdResult<Price> {
        match self {
            OracleContext::Local { storage } => {
                PYTH_PRICES.load(*storage, pyth_id)
            },
            OracleContext::Remote { address, querier } => {
                querier.query_wasm_path(*address, &PYTH_PRICES.path(pyth_id))
            },
        }
    }

    fn get_price_source(&self, denom: &Denom) -> StdResult<PriceSource> {
        match self {
            OracleContext::Local { storage } => {
                PRICE_SOURCES.load(*storage, denom)
            },
            OracleContext::Remote { address, querier } => {
                querier.query_wasm_path(*address, &PRICE_SOURCES.path(denom))
            },
        }
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_order_book::UsdPrice,
        dango_types::constants::{eth, usdc},
        grug_types::{ResultExt, Timestamp, hash_map},
        pyth_types::MarketSession,
        test_case::test_case,
    };

    #[test_case(
        hash_map! {
            eth::DENOM.clone() => Price::new(
                UsdPrice::new_int(2_000),
                Timestamp::from_seconds(1730802926),
                MarketSession::Regular,
            ),
        };
        "mock with one price"
    )]
    #[test_case(
        hash_map! {
            eth::DENOM.clone() => Price::new(
                UsdPrice::new_int(2_000),
                Timestamp::from_seconds(1730802926),
                MarketSession::Regular,
            ),
            usdc::DENOM.clone() => Price::new(
                UsdPrice::new_int(1),
                Timestamp::from_seconds(1730802926),
                MarketSession::Regular,
            ),
        };
        "mock with two prices"
    )]
    fn mock(prices: HashMap<Denom, Price>) {
        let mut oracle_querier = OracleQuerier::new_mock(prices.clone());

        for (denom, expected_price) in prices {
            oracle_querier
                .query_price(&denom, None)
                .should_succeed_and_equal(expected_price);
        }
    }

    #[test]
    fn mock_querier_with_no_prices() {
        let mut oracle_querier = OracleQuerier::new_mock(HashMap::new());

        oracle_querier
            .query_price(&eth::DENOM, None)
            .should_fail_with_error(format!(
                "price not provided to oracle querier for denom `{}`",
                eth::DENOM.clone()
            ));
    }
}
