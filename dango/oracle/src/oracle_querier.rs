use {
    crate::PRICE_SOURCES,
    anyhow::anyhow,
    dango_types::oracle::{PrecisionedPrice, PrecisionlessPrice, PriceSource, PRICES},
    grug::{Addr, BorshDeExt, Denom, Querier, QuerierExt, StdError},
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
        let price_source = self
            .query_wasm_raw(oracle, PRICE_SOURCES.path(denom))?
            .ok_or(anyhow!("price source not found for denom `{denom}`"))?
            .deserialize_borsh::<PriceSource>()?;

        match price_source {
            PriceSource::Pyth { id, precision } => {
                let price = self
                    .query_wasm_raw(oracle, PRICES.path(id))?
                    .ok_or(anyhow!("price not found for pyth id: {id}"))?
                    .deserialize_borsh::<PrecisionlessPrice>()?
                    .with_precision(precision);
                Ok(price)
            },
        }
    }
}
