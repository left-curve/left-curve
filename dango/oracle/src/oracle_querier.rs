use {
    crate::{PRICE_SOURCES, PRICES, PYTH_LAZER_PRICES},
    anyhow::{anyhow, ensure},
    dango_types::{
        DangoQuerier,
        lending::{NAMESPACE, SUBNAMESPACE},
        oracle::{PrecisionedPrice, PrecisionlessPrice, PriceSource},
    },
    grug::{
        Addr, Cache, Denom, Number, QuerierWrapper, StdResult, Storage, StorageQuerier, Timestamp,
        Udec128,
    },
    pyth_types::{PythId, PythLazerId},
    std::{cell::OnceCell, collections::HashMap},
};

pub struct OracleQuerier<'a> {
    cache: Cache<'a, Denom, PrecisionedPrice, anyhow::Error, PriceSource>,
    no_older_than: Option<Timestamp>,
}

impl<'a> OracleQuerier<'a> {
    /// Create a new `OracleQuerier` for in another contract, with caching.
    pub fn new_remote(address: Addr, querier: QuerierWrapper<'a>) -> Self {
        let ctx = OracleContext::Remote { address, querier };
        let no_cache_querier = OracleQuerierNoCache::new(ctx, querier);

        Self {
            cache: Cache::new(move |denom, price_source| {
                no_cache_querier.query_price(denom, price_source)
            }),
            no_older_than: None,
        }
    }

    /// Create a new `OracleQuerier` that returns predefined prices in a hash map.
    /// For using in tests.
    pub fn new_mock(prices: HashMap<Denom, PrecisionedPrice>) -> Self {
        Self {
            cache: Cache::new(move |denom, _| {
                prices.get(denom).cloned().ok_or_else(|| {
                    anyhow!("[mock]: price not provided to oracle querier for denom `{denom}`")
                })
            }),
            no_older_than: None,
        }
    }

    pub fn with_no_older_than(mut self, no_older_than: Timestamp) -> Self {
        self.no_older_than = Some(no_older_than);
        self
    }

    pub fn query_price(
        &mut self,
        denom: &Denom,
        price_source: Option<PriceSource>,
    ) -> anyhow::Result<PrecisionedPrice> {
        self.cache
            .get_or_fetch(denom, price_source)
            .and_then(|price| {
                if let Some(no_older_than) = self.no_older_than {
                    ensure!(
                        price.timestamp >= no_older_than,
                        "price is too old! denom: {}, timestamp: {}, must be no older than: {}",
                        denom,
                        price.timestamp.into_nanos(),
                        no_older_than.into_nanos()
                    );
                }

                Ok(price)
            })
            .cloned()
    }
}

pub(crate) struct OracleQuerierNoCache<'a> {
    ctx: OracleContext<'a>,
    lending: RemoteLending<'a>,
}

impl<'a> OracleQuerierNoCache<'a> {
    /// Create a new `OracleQuerierNoCache` for use inside the oracle contract
    /// itself.
    pub fn new_local(storage: &'a dyn Storage, querier: QuerierWrapper<'a>) -> Self {
        Self::new(OracleContext::Local { storage }, querier)
    }

    fn new(ctx: OracleContext<'a>, querier: QuerierWrapper<'a>) -> Self {
        Self {
            ctx,
            lending: RemoteLending::new(querier),
        }
    }

    pub fn query_price(
        &self,
        denom: &Denom,
        price_source: Option<PriceSource>,
    ) -> anyhow::Result<PrecisionedPrice> {
        // Query the denom's price source, if not provided.
        let price_source = price_source.map_or_else(|| self.ctx.get_price_source(denom), Ok)?;

        // Compute the price based on the price source.
        match price_source {
            PriceSource::Fixed {
                humanized_price,
                precision,
                timestamp,
            } => {
                let price = PrecisionlessPrice::new(humanized_price, timestamp);
                Ok(price.with_precision(precision))
            },
            PriceSource::Pyth { id, precision } => {
                let (price, _) = self.ctx.get_price(id)?;
                Ok(price.with_precision(precision))
            },
            PriceSource::PythLazer { id, precision, .. } => {
                let price = self.ctx.get_lazer_price(id)?;
                Ok(price.with_precision(precision))
            },
            PriceSource::LendingLiquidity => {
                // Get the underlying denom.
                let underlying_denom =
                    denom.strip(&[&NAMESPACE, &SUBNAMESPACE]).ok_or_else(|| {
                        anyhow!(
                            "not a lending pool token: `{denom}`! must start with `{}/{}`",
                            NAMESPACE.as_ref(),
                            SUBNAMESPACE.as_ref()
                        )
                    })?;

                // Get the price of the underlying asset.
                let underlying_price = self.query_price(&underlying_denom, None)?;

                // Get supply index of the LP token.
                let supply_index = self.lending.get_supply_index(&underlying_denom)?;

                // Calculate the price of the LP token.
                Ok(PrecisionedPrice::new(
                    underlying_price.humanized_price.checked_mul(supply_index)?,
                    underlying_price.timestamp,
                    underlying_price.precision(),
                ))
            },
        }
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
    fn get_price(&self, pyth_id: PythId) -> StdResult<(PrecisionlessPrice, u64)> {
        match self {
            OracleContext::Local { storage } => {
                PRICES.load(*storage, pyth_id)
            },
            OracleContext::Remote { address, querier } => {
                querier.query_wasm_path(*address, &PRICES.path(pyth_id))
            },
        }
    }

    fn get_lazer_price(&self, lazer_id: PythLazerId) -> StdResult<PrecisionlessPrice> {
        match self {
            OracleContext::Local { storage } => {
                PYTH_LAZER_PRICES.load(*storage, lazer_id)
            },
            OracleContext::Remote { address, querier } => {
                querier.query_wasm_path(*address, &PYTH_LAZER_PRICES.path(lazer_id))
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

struct RemoteLending<'a> {
    // TODO: Change this to a `OnceCell<Addr>` and use `get_or_try_init` when
    // the feature is stablized.
    address: OnceCell<StdResult<Addr>>,
    querier: QuerierWrapper<'a>,
}

impl<'a> RemoteLending<'a> {
    pub fn new(querier: QuerierWrapper<'a>) -> Self {
        Self {
            address: OnceCell::new(),
            querier,
        }
    }

    pub fn get_address(&self) -> StdResult<Addr> {
        self.address
            .get_or_init(|| {
                let cfg = self.querier.query_dango_config()?;
                Ok(cfg.addresses.lending)
            })
            .clone()
    }

    pub fn get_supply_index(&self, underlying_denom: &Denom) -> StdResult<Udec128> {
        self.querier
            .query_wasm_path(
                self.get_address()?,
                &dango_lending::MARKETS.path(underlying_denom),
            )
            .map(|market| market.supply_index)
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_types::constants::{eth, usdc},
        grug::{ResultExt, Timestamp, hash_map},
        test_case::test_case,
    };

    #[test_case(
        hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(2000),
                Timestamp::from_seconds(1730802926),
                6,
            ),
        };
        "mock with one price"
    )]
    #[test_case(
        hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(2000),
                Timestamp::from_seconds(1730802926),
                6,
            ),
            usdc::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(1000),
                Timestamp::from_seconds(1730802926),
                6,
            ),
        };
        "mock with two prices"
    )]
    fn mock(prices: HashMap<Denom, PrecisionedPrice>) {
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

    #[test_case(
        Timestamp::from_seconds(1730802926), None => true;
        "`no_older_than` is unspecified; should succeed"
    )]
    #[test_case(
        Timestamp::from_seconds(1730802926), Some(Timestamp::from_seconds(1730802925)) => true;
        "`no_older_than` is older than the price timestamp; should succeed"
    )]
    #[test_case(
        Timestamp::from_seconds(1730802926), Some(Timestamp::from_seconds(1730802926)) => true;
        "`no_older_than` equals the price timestamp; should succeed"
    )]
    #[test_case(
        Timestamp::from_seconds(1730802926), Some(Timestamp::from_seconds(1730802927)) => false;
        "`no_older_than` is newer than the price timestamp; should fail"
    )]
    fn querier_staleness_assertion_works(
        publish_time: Timestamp,
        no_older_than: Option<Timestamp>,
    ) -> bool {
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(2000),
                publish_time,
                6,
            ),
        });

        if let Some(no_older_than) = no_older_than {
            oracle_querier = oracle_querier.with_no_older_than(no_older_than);
        }

        oracle_querier.query_price(&eth::DENOM, None).is_ok()
    }
}
