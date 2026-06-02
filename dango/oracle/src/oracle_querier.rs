use {
    crate::{PRICE_SOURCES, PYTH_PRICES},
    anyhow::anyhow,
    dango_order_book::{Dimensionless, UsdPrice},
    dango_types::oracle::{Price, PriceSourceWithWeight},
    grug_storage::StorageQuerier,
    grug_types::{Addr, Cache, Denom, QuerierWrapper, StdResult, Storage},
    pyth_types::{MarketSession, PythId},
    std::collections::HashMap,
};

pub struct OracleQuerier<'a> {
    cache: Cache<'a, Denom, Price, anyhow::Error, Vec<PriceSourceWithWeight>>,
}

impl<'a> OracleQuerier<'a> {
    /// Create a new `OracleQuerier` for in another contract, with caching.
    pub fn new_remote(address: Addr, querier: QuerierWrapper<'a>) -> Self {
        let ctx = OracleContext::Remote { address, querier };
        let no_cache_querier = OracleQuerierNoCache::new(ctx);

        Self {
            cache: Cache::new(move |denom, price_sources| {
                no_cache_querier.query_price(denom, price_sources)
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
        price_sources: Option<Vec<PriceSourceWithWeight>>,
    ) -> anyhow::Result<Price> {
        self.cache.get_or_fetch(denom, price_sources).cloned()
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

    /// Query the combined price of a denom from its (possibly several) weighted
    /// price sources.
    ///
    /// A denom priced from a single source returns that source's price. A denom
    /// priced from several sources (e.g. a commodity rolling between two futures
    /// contracts) returns the weight-normalized arithmetic mean of the
    /// components, with:
    ///
    /// - timestamp = the **oldest** component timestamp, so a single stale feed
    ///   makes the whole price stale (the perps contract then falls back to its
    ///   internal order-book price);
    /// - market session = `Regular` only if **every** component is in regular
    ///   session, else `Other`.
    pub fn query_price(
        &self,
        denom: &Denom,
        price_sources: Option<Vec<PriceSourceWithWeight>>,
    ) -> anyhow::Result<Price> {
        // Query the denom's price sources, if not provided.
        let price_sources = price_sources.map_or_else(|| self.ctx.get_price_source(denom), Ok)?;

        // Fetch each component price. If any component has no on-chain price yet
        // (or otherwise fails to load), the whole query fails -- the perps
        // contract treats a failed oracle read as "price unavailable" and falls
        // back to its internal order-book price.
        let components = price_sources
            .iter()
            .map(|p| Ok((self.ctx.get_price(p.price_source.id)?, p.weight)))
            .collect::<anyhow::Result<Vec<(Price, Dimensionless)>>>()?;

        // `RegisterPriceSources` and `instantiate` guarantee a non-empty list
        // with strictly positive weights (hence a positive total weight), so we
        // combine the components without re-validating here. `checked_div` still
        // surfaces a zero divisor as an error rather than panicking, should a
        // malformed entry ever reach storage.
        let total_weight = components
            .iter()
            .try_fold(Dimensionless::ZERO, |acc, (_, weight)| {
                acc.checked_add(*weight)
            })?;

        let weighted_sum = components
            .iter()
            .try_fold(UsdPrice::ZERO, |acc, (price, weight)| {
                acc.checked_add(price.humanized_price.checked_mul(*weight)?)
            })?;

        let humanized_price = weighted_sum.checked_div(total_weight)?;

        // The combined price is only as fresh as its oldest component.
        let timestamp = components
            .iter()
            .map(|(price, _)| price.timestamp)
            .min()
            .ok_or_else(|| anyhow!("no price sources for denom `{denom}`"))?;

        // The market is in regular session only if every component is.
        let market_session = if components
            .iter()
            .all(|(price, _)| price.market_session == MarketSession::Regular)
        {
            MarketSession::Regular
        } else {
            MarketSession::Other
        };

        Ok(Price::new(humanized_price, timestamp, market_session))
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

    fn get_price_source(&self, denom: &Denom) -> StdResult<Vec<PriceSourceWithWeight>> {
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
        dango_types::{
            constants::{eth, perp_btc, usdc},
            oracle::PriceSource,
        },
        grug_types::{MockStorage, ResultExt, Timestamp, hash_map},
        pyth_types::Channel,
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

    /// Seed `PRICE_SOURCES` and `PYTH_PRICES` for `denom` and query the combined
    /// price via the local (in-contract) querier.
    fn query_local(
        denom: &Denom,
        sources: Vec<(PythId, Dimensionless)>,
        prices: Vec<(PythId, Price)>,
    ) -> anyhow::Result<Price> {
        let mut storage = MockStorage::default();

        let price_sources = sources
            .into_iter()
            .map(|(id, weight)| PriceSourceWithWeight {
                price_source: PriceSource {
                    id,
                    channel: Channel::RealTime,
                },
                weight,
            })
            .collect::<Vec<_>>();
        PRICE_SOURCES
            .save(&mut storage, denom, &price_sources)
            .unwrap();

        for (id, price) in prices {
            PYTH_PRICES.save(&mut storage, id, &price).unwrap();
        }

        OracleQuerierNoCache::new_local(&storage).query_price(denom, None)
    }

    /// A single source returns its own price regardless of the weight.
    #[test]
    fn single_source_returns_its_price() {
        query_local(&eth::DENOM, vec![(1, Dimensionless::new_int(1))], vec![(
            1,
            Price::new(
                UsdPrice::new_int(2_000),
                Timestamp::from_seconds(1_000),
                MarketSession::Regular,
            ),
        )])
        .should_succeed_and_equal(Price::new(
            UsdPrice::new_int(2_000),
            Timestamp::from_seconds(1_000),
            MarketSession::Regular,
        ));
    }

    /// Two sources with unequal weights: 100 * 0.75 + 200 * 0.25 = 125. The
    /// combined timestamp is the oldest (900), and the session stays `Regular`
    /// because both components are.
    #[test]
    fn two_sources_weighted_mean() {
        query_local(
            &perp_btc::DENOM,
            vec![
                (1, Dimensionless::new_percent(75)),
                (2, Dimensionless::new_percent(25)),
            ],
            vec![
                (
                    1,
                    Price::new(
                        UsdPrice::new_int(100),
                        Timestamp::from_seconds(1_000),
                        MarketSession::Regular,
                    ),
                ),
                (
                    2,
                    Price::new(
                        UsdPrice::new_int(200),
                        Timestamp::from_seconds(900),
                        MarketSession::Regular,
                    ),
                ),
            ],
        )
        .should_succeed_and_equal(Price::new(
            UsdPrice::new_int(125),
            Timestamp::from_seconds(900),
            MarketSession::Regular,
        ));
    }

    /// Equal weights average the two component prices: (100 + 200) / 2 = 150.
    #[test]
    fn two_sources_equal_weights() {
        query_local(
            &perp_btc::DENOM,
            vec![
                (1, Dimensionless::new_int(1)),
                (2, Dimensionless::new_int(1)),
            ],
            vec![
                (
                    1,
                    Price::new(
                        UsdPrice::new_int(100),
                        Timestamp::from_seconds(1_000),
                        MarketSession::Regular,
                    ),
                ),
                (
                    2,
                    Price::new(
                        UsdPrice::new_int(200),
                        Timestamp::from_seconds(1_000),
                        MarketSession::Regular,
                    ),
                ),
            ],
        )
        .should_succeed_and_equal(Price::new(
            UsdPrice::new_int(150),
            Timestamp::from_seconds(1_000),
            MarketSession::Regular,
        ));
    }

    /// If any component is not in regular session, the combined session is
    /// `Other`; the combined timestamp is still the oldest component's.
    #[test]
    fn non_regular_if_any_component_is_non_regular() {
        query_local(
            &perp_btc::DENOM,
            vec![
                (1, Dimensionless::new_int(1)),
                (2, Dimensionless::new_int(1)),
            ],
            vec![
                (
                    1,
                    Price::new(
                        UsdPrice::new_int(100),
                        Timestamp::from_seconds(1_000),
                        MarketSession::Regular,
                    ),
                ),
                (
                    2,
                    Price::new(
                        UsdPrice::new_int(200),
                        Timestamp::from_seconds(800),
                        MarketSession::Other,
                    ),
                ),
            ],
        )
        .should_succeed_and_equal(Price::new(
            UsdPrice::new_int(150),
            Timestamp::from_seconds(800),
            MarketSession::Other,
        ));
    }

    /// If a component feed has no on-chain price yet, the whole query fails.
    #[test]
    fn missing_component_price_fails() {
        query_local(
            &perp_btc::DENOM,
            vec![
                (1, Dimensionless::new_int(1)),
                (2, Dimensionless::new_int(1)),
            ],
            // Only feed `1`; feed `2` has no price.
            vec![(
                1,
                Price::new(
                    UsdPrice::new_int(100),
                    Timestamp::from_seconds(1_000),
                    MarketSession::Regular,
                ),
            )],
        )
        .should_fail();
    }
}
