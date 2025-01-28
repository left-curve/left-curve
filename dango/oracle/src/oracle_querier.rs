use {
    crate::PRICE_SOURCES,
    dango_types::oracle::{PrecisionedPrice, PrecisionlessPrice, PriceSource, PRICES},
    grug::{Addr, Denom, Querier, StdError, StorageQuerier},
};

/// A trait for querying prices from the oracle.
pub trait OracleQuerier: Querier {
    /// Queries the price for a given denom from the oracle.
    fn query_price(&self, oracle: Addr, denom: &Denom) -> anyhow::Result<PrecisionedPrice>;
}

impl<Q> OracleQuerier for Q
where
    Q: Querier,
    Q::Error: From<StdError>,
    anyhow::Error: From<Q::Error>,
{
    fn query_price(&self, oracle: Addr, denom: &Denom) -> anyhow::Result<PrecisionedPrice> {
        match self.query_wasm_path(oracle, &PRICE_SOURCES.path(denom))? {
            PriceSource::Fixed {
                humanized_price,
                precision,
                timestamp,
            } => {
                let price = PrecisionlessPrice::new(humanized_price, humanized_price, timestamp);
                Ok(price.with_precision(precision))
            },
            PriceSource::Pyth { id, precision } => {
                let price = self.query_wasm_path(oracle, &PRICES.path(id))?;
                Ok(price.with_precision(precision))
            },
        }
    }
}
