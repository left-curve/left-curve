use std::collections::HashMap;

use grug::{Denom, Int128, Number, NumberConst, Udec128, Uint128};

use super::{PerpsMarketParams, PerpsMarketState, RealisedCashFlow};

/// The state of the perps vault
#[grug::derive(Serde, Borsh)]
pub struct PerpsVaultState {
    /// The denom that is deposited into the vault.
    pub denom: Denom,
    /// The amount of the denom that is deposited into the vault.
    pub deposits: Uint128,
    /// The amount of shares that that have been minted.
    pub shares: Uint128,
    /// The realised cash flow of the vault.
    pub realised_cash_flow: RealisedCashFlow,
}

impl PerpsVaultState {
    pub fn net_asset_value(
        &self,
        markets: &[PerpsMarketState],
        params: &HashMap<Denom, PerpsMarketParams>,
        oracle_prices: &HashMap<Denom, Udec128>,
    ) -> anyhow::Result<Int128> {
        Ok(markets
            .iter()
            .map(|market| {
                let params = params.get(&market.denom).ok_or(anyhow::anyhow!(
                    "params not found for denom: {}",
                    market.denom
                ))?;
                let oracle_price = oracle_prices
                    .get(&market.denom)
                    .ok_or(anyhow::anyhow!(
                        "oracle price not found for denom: {}",
                        market.denom
                    ))?
                    .clone();
                market.net_asset_value(params, oracle_price)
            })
            .try_fold(Int128::ZERO, |acc, x| {
                Ok::<_, anyhow::Error>(acc.checked_add(x?)?)
            })?)
    }
}
