use {
    crate::{PRICE_SOURCES, PRICES},
    anyhow::anyhow,
    dango_types::{
        config::AppConfig,
        lending::{NAMESPACE, SUBNAMESPACE},
        oracle::{PrecisionedPrice, PrecisionlessPrice, PriceSource},
    },
    grug::{Addr, Denom, Number, Querier, QuerierExt, StorageQuerier},
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
            PriceSource::LendingLiquidity => {
                // Get the price of the underlying asset
                let underlying_denom = denom
                    .strip(&[&NAMESPACE, &SUBNAMESPACE])
                    .ok_or_else(|| anyhow!("not a lending pool token: {denom}"))?;
                let underlying_price = self.query_price(oracle, &underlying_denom, None)?;

                // Get supply index of the LP token
                let app_cfg: AppConfig = self.query_app_config()?;
                let supply_index = self
                    .query_wasm_path(
                        app_cfg.addresses.lending,
                        &dango_lending::MARKETS.path(&underlying_denom),
                    )?
                    .supply_index;

                // Calculate the price of the LP token
                Ok(PrecisionedPrice::new(
                    underlying_price.humanized_price.checked_mul(supply_index)?,
                    underlying_price.humanized_ema.checked_mul(supply_index)?,
                    underlying_price.timestamp,
                    underlying_price.precision(),
                ))
            },
        }
    }
}
