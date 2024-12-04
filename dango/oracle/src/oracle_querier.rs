use {
    crate::PRICE_SOURCES,
    dango_types::oracle::{PrecisionedPrice, PriceSource, PRICES},
    grug::{Addr, BorshDeExt, Denom, PathQuerier, QuerierWrapper},
};

/// A trait for querying prices from the oracle.
pub trait OracleQuerier {
    /// Queries the price for a given denom from the oracle.
    fn query_price(&self, oracle: Addr, denom: &Denom) -> anyhow::Result<PrecisionedPrice>;
}

impl OracleQuerier for QuerierWrapper<'_> {
    fn query_price(&self, oracle: Addr, denom: &Denom) -> anyhow::Result<PrecisionedPrice> {
        let price_source = self
            .query_wasm_raw(oracle, PRICE_SOURCES.path(denom))?
            .ok_or(anyhow::anyhow!(
                "price source not found for denom `{denom}`"
            ))?
            .deserialize_borsh::<PriceSource>()?;

        match price_source {
            PriceSource::Pyth { id, precision } => {
                let price = self
                    .query_wasm_path(oracle, PRICES.path(id))?
                    .with_precision(precision);
                Ok(price)
            },
        }
    }
}
