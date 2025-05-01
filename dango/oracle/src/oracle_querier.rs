use {
    crate::{PRICE_SOURCES, PRICES},
    dango_types::oracle::{PrecisionedPrice, PrecisionlessPrice, PriceSource},
    grug::{Addr, Denom, Querier, StorageQuerier},
};

/// A trait for querying prices from the oracle.
pub trait OracleQuerier: Querier {
    /// Queries the price for a given denom from the oracle.
    fn query_price(
        &self,
        oracle: Addr,
        denom: &Denom,
        price_source: Option<PriceSource>,
    ) -> anyhow::Result<PrecisionedPrice>;
}

impl<Q> OracleQuerier for Q
where
    Q: Querier,
{
    fn query_price(
        &self,
        oracle: Addr,
        denom: &Denom,
        price_source: Option<PriceSource>,
    ) -> anyhow::Result<PrecisionedPrice> {
        let price_source = match price_source {
            Some(source) => source,
            None => self.query_wasm_path(oracle, &PRICE_SOURCES.path(denom))?,
        };

        match price_source {
            PriceSource::Fixed {
                humanized_price,
                precision,
                timestamp,
            } => {
                let price = PrecisionlessPrice::new(humanized_price, humanized_price, timestamp);
                Ok(price.with_precision(precision))
            },
            PriceSource::Pyth { id, precision } => {
                let price = self.query_wasm_path(oracle, &PRICES.path(id))?.0;
                Ok(price.with_precision(precision))
            },
        }
    }
}
