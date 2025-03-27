use {
    crate::{PRICE_SOURCES, PRICES},
    anyhow::anyhow,
    dango_types::{
        config::AppConfig,
        lending::{NAMESPACE, SUBNAMESPACE},
        oracle::{PrecisionedPrice, PrecisionlessPrice, PriceSource},
    },
    grug::{Addr, Denom, Number, QuerierExt, StorageQuerier},
    std::collections::HashMap,
};

pub struct OracleQuerier {
    oracle: Addr,
    cache: HashMap<Denom, PrecisionedPrice>,
}

impl OracleQuerier {
    pub fn new(oracle: Addr) -> Self {
        Self {
            oracle,
            cache: HashMap::new(),
        }
    }

    pub fn query_price<Q>(
        &mut self,
        querier: &Q,
        denom: &Denom,
        price_source: Option<PriceSource>,
    ) -> anyhow::Result<PrecisionedPrice>
    where
        Q: QuerierExt,
    {
        let price = self.cache.get(denom);
        if let Some(price) = price {
            return Ok(price.clone());
        }

        let price_source = match price_source {
            Some(source) => source,
            None => querier.query_wasm_path(self.oracle, &PRICE_SOURCES.path(denom))?,
        };

        let price = match price_source {
            PriceSource::Fixed {
                humanized_price,
                precision,
                timestamp,
            } => {
                let price = PrecisionlessPrice::new(humanized_price, humanized_price, timestamp);
                Ok::<_, anyhow::Error>(price.with_precision(precision))
            },
            PriceSource::Pyth { id, precision } => {
                let price = querier.query_wasm_path(self.oracle, &PRICES.path(id))?.0;
                Ok(price.with_precision(precision))
            },
            PriceSource::LendingLiquidity => {
                // Get the price of the underlying asset
                let underlying_denom = denom
                    .strip(&[&NAMESPACE, &SUBNAMESPACE])
                    .ok_or_else(|| anyhow!("not a lending pool token: {denom}"))?;
                let underlying_price = self.query_price(querier, &underlying_denom, None)?;

                // Get supply index of the LP token
                let app_cfg: AppConfig = querier.query_app_config()?;
                let supply_index = querier
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
        }?;

        self.cache.insert(denom.clone(), price.clone());

        Ok(price)
    }
}
