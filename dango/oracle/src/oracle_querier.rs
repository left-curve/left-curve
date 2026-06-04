use {
    crate::{PRICE_SOURCES, PYTH_PRICES},
    anyhow::{anyhow, ensure},
    dango_order_book::UsdPrice,
    dango_types::oracle::{Price, PriceConfig},
    grug_storage::StorageQuerier,
    grug_types::{Addr, Cache, Denom, QuerierWrapper, StdResult, Storage, Timestamp},
    pyth_types::{MarketSession, PythId},
    std::collections::HashMap,
};

pub struct OracleQuerier<'a> {
    cache: Cache<'a, Denom, Price, anyhow::Error, PriceConfig>,
}

impl<'a> OracleQuerier<'a> {
    /// Create a new `OracleQuerier` for use in another contract, with caching.
    ///
    /// `now` is the current block timestamp, used to evaluate time-weighted
    /// (futures roll) price configs.
    pub fn new_remote(address: Addr, querier: QuerierWrapper<'a>, now: Timestamp) -> Self {
        let ctx = OracleContext::Remote { address, querier };
        let no_cache_querier = OracleQuerierNoCache::new(ctx, now);

        Self {
            cache: Cache::new(move |denom, config| no_cache_querier.query_price(denom, config)),
        }
    }

    /// Create a new `OracleQuerier` that returns predefined prices in a hash map.
    /// For using in tests.
    pub fn new_mock(prices: HashMap<Denom, Price>) -> Self {
        Self {
            cache: Cache::new(move |denom, _: Option<PriceConfig>| {
                prices.get(denom).cloned().ok_or_else(|| {
                    anyhow!("[mock]: price not provided to oracle querier for denom `{denom}`")
                })
            }),
        }
    }

    pub fn query_price(
        &mut self,
        denom: &Denom,
        config: Option<PriceConfig>,
    ) -> anyhow::Result<Price> {
        self.cache.get_or_fetch(denom, config).cloned()
    }
}

pub(crate) struct OracleQuerierNoCache<'a> {
    ctx: OracleContext<'a>,
    now: Timestamp,
}

impl<'a> OracleQuerierNoCache<'a> {
    /// Create a new `OracleQuerierNoCache` for use inside the oracle contract
    /// itself. `now` is the current block timestamp.
    pub fn new_local(storage: &'a dyn Storage, now: Timestamp) -> Self {
        Self::new(OracleContext::Local { storage }, now)
    }

    fn new(ctx: OracleContext<'a>, now: Timestamp) -> Self {
        Self { ctx, now }
    }

    /// Query the combined price of a denom from its price config.
    ///
    /// A single-source denom returns that feed's price. A futures-roll denom
    /// returns the weighted blend of its active contracts at the current block
    /// timestamp, with:
    ///
    /// - timestamp = the **oldest** component timestamp, so a single stale feed
    ///   makes the whole price stale (the consumer then falls back to its own
    ///   internal price);
    /// - market session = `Regular` only if **every** component is in regular
    ///   session, else `Other`.
    ///
    /// If any component has no on-chain price yet, the whole query fails.
    pub fn query_price(&self, denom: &Denom, config: Option<PriceConfig>) -> anyhow::Result<Price> {
        // Load the denom's price config, unless one was provided by the caller.
        let config = config.map_or_else(|| self.ctx.get_price_config(denom), Ok)?;

        // Resolve the feeds to blend at the current time: one for a single-source
        // denom, one or two during a futures roll. The weights sum to one by
        // construction, so the weighted sum needs no normalizing division.
        let components = config.components_at(self.now)?;

        let mut humanized_price = UsdPrice::ZERO;
        let mut timestamp: Option<Timestamp> = None;
        let mut all_regular = true;

        for (source, weight) in &components {
            let price = self.ctx.get_price(source.id)?;

            humanized_price =
                humanized_price.checked_add(price.humanized_price.checked_mul(*weight)?)?;

            // The combined price is only as fresh as its oldest component.
            timestamp = Some(match timestamp {
                Some(current) => current.min(price.timestamp),
                None => price.timestamp,
            });

            // The market is in regular session only if every component is.
            all_regular &= price.market_session == MarketSession::Regular;
        }

        // Reject a non-positive blended price (e.g. a Pyth feed reporting a
        // negative commodity price). The consumer treats a failed oracle read as
        // "price unavailable" and falls back to its internal price, so erroring
        // here is fail-closed.
        ensure!(
            humanized_price.is_positive(),
            "combined price for denom `{denom}` is non-positive: {humanized_price}"
        );

        let timestamp =
            timestamp.ok_or_else(|| anyhow!("no price components for denom `{denom}`"))?;

        let market_session = if all_regular {
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

    fn get_price_config(&self, denom: &Denom) -> StdResult<PriceConfig> {
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
        dango_order_book::{Dimensionless, UsdPrice},
        dango_types::{
            constants::{eth, perp_btc, usdc},
            oracle::{Fixing, PriceSource, RollState},
        },
        grug_types::{MockStorage, ResultExt, Timestamp, hash_map},
        pyth_types::{Channel, MarketSession},
        std::collections::VecDeque,
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

    fn source(id: u32) -> PriceSource {
        PriceSource {
            id,
            channel: Channel::RealTime,
        }
    }

    /// Seed `PRICE_SOURCES` and `PYTH_PRICES`, then query via the local querier
    /// at `now`.
    fn query_local(
        denom: &Denom,
        config: PriceConfig,
        prices: Vec<(u32, Price)>,
        now: u128,
    ) -> anyhow::Result<Price> {
        let mut storage = MockStorage::default();
        PRICE_SOURCES.save(&mut storage, denom, &config).unwrap();
        for (id, price) in prices {
            PYTH_PRICES.save(&mut storage, id, &price).unwrap();
        }
        OracleQuerierNoCache::new_local(&storage, Timestamp::from_seconds(now))
            .query_price(denom, None)
    }

    fn price(value: i128, secs: u128, session: MarketSession) -> Price {
        Price::new(
            UsdPrice::new_int(value),
            Timestamp::from_seconds(secs),
            session,
        )
    }

    /// A single-source denom returns its own price unchanged, at any time.
    #[test]
    fn single_source_passthrough() {
        query_local(
            &eth::DENOM,
            PriceConfig::single(source(1)),
            vec![(1, price(2_000, 1_000, MarketSession::Regular))],
            9_999,
        )
        .should_succeed_and_equal(price(2_000, 1_000, MarketSession::Regular));
    }

    /// A futures roll blends the two contracts by the weight in force at `now`.
    /// At 40% on `next`: 100 * 0.6 + 120 * 0.4 = 108. Timestamp is the oldest
    /// component (900); session is `Regular` since both are.
    #[test]
    fn roll_blends_at_current_weight() {
        let roll = PriceConfig::Roll(RollState {
            current: source(1),
            next: source(2),
            fixings: vec![
                Fixing {
                    at: Timestamp::from_seconds(100),
                    next_weight: Dimensionless::new_percent(20),
                },
                Fixing {
                    at: Timestamp::from_seconds(200),
                    next_weight: Dimensionless::new_percent(40),
                },
                Fixing {
                    at: Timestamp::from_seconds(300),
                    next_weight: Dimensionless::new_percent(100),
                },
            ],
            upcoming: VecDeque::new(),
        });

        query_local(
            &perp_btc::DENOM,
            roll,
            vec![
                (1, price(100, 1_000, MarketSession::Regular)),
                (2, price(120, 900, MarketSession::Regular)),
            ],
            200, // 40% weight on `next`
        )
        .should_succeed_and_equal(price(108, 900, MarketSession::Regular));
    }

    /// A non-positive blended price (here a negative component) is rejected.
    #[test]
    fn rejects_non_positive_price() {
        query_local(
            &eth::DENOM,
            PriceConfig::single(source(1)),
            vec![(1, price(-5, 1_000, MarketSession::Regular))],
            1_000,
        )
        .should_fail_with_error("non-positive");
    }
}
