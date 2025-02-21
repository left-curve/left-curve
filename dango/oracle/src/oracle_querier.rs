use {
    crate::PRICE_SOURCES,
    anyhow::{anyhow, bail},
    dango_types::{
        config::AppConfig,
        dex::CurveInvariant,
        lending::{NAMESPACE, SUBNAMESPACE},
        oracle::{PrecisionedPrice, PrecisionlessPrice, PriceSource, PRICES},
    },
    grug::{
        Addr, Denom, Number, NumberConst, Querier, QuerierExt, StdError, StorageQuerier, Udec128,
    },
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
    Q: QuerierExt,
    Q::Error: From<StdError>,
    anyhow::Error: From<Q::Error>,
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
                let price = self.query_wasm_path(oracle, &PRICES.path(id))?;
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
            PriceSource::PassiveLiquidity => {
                let app_cfg: AppConfig = self.query_app_config()?;
                let pool =
                    self.query_wasm_path(app_cfg.addresses.dex, &dango_dex::POOLS.path(denom))?;

                match pool.curve_type {
                    CurveInvariant::Xyk => {
                        // Calculate the fair LP price according to https://blog.alphaventuredao.io/fair-lp-token-pricing/
                        let underlying_prices = pool
                            .reserves
                            .clone()
                            .into_iter()
                            .map(|coin| self.query_price(oracle, &coin.denom, None))
                            .collect::<anyhow::Result<Vec<_>>>()?;

                        // Get the oldest price's timestamp to use for this price's timestamp
                        let oldest_price_timestamp = underlying_prices
                            .iter()
                            .map(|price| price.timestamp)
                            .min()
                            .ok_or_else(|| {
                                anyhow!(
                                    "No prices found for underlying assets of LP token: {denom}"
                                )
                            })?;

                        // Calculates the product of the prices.
                        let prices_product = underlying_prices
                            .iter()
                            .fold(Ok(Udec128::ONE), |acc, price| {
                                acc?.checked_mul(price.humanized_price)
                            })?;

                        // Calculates the invariant of the pool.
                        let invariant = pool
                            .reserves
                            .into_iter()
                            .map(|coin| coin.amount)
                            .fold(Ok(Udec128::ONE), |acc, amount| {
                                acc?.checked_mul(amount.checked_into_dec()?)
                            })?;

                        // Finally calculate the fair price of the LP token for each micro unit of the
                        // LP token.
                        let lp_supply = self.query_supply(denom.clone())?;
                        let two = Udec128::ONE + Udec128::ONE;
                        let fair_lp_price = two
                            .checked_mul(prices_product.checked_mul(invariant)?.checked_sqrt()?)?
                            .checked_div(lp_supply.checked_into_dec()?)?;

                        // Convert the fair price to a humanized price with 6 decimal places.
                        let precision = 6u8;
                        let humanized_price = fair_lp_price
                            .checked_mul(Udec128::new(10).checked_pow(precision as u32)?)?;

                        Ok(PrecisionedPrice::new(
                            humanized_price,
                            humanized_price,
                            oldest_price_timestamp,
                            precision,
                        ))
                    },
                    _ => bail!("unsupported curve type: {:?}", pool.curve_type),
                }
            },
        }
    }
}
