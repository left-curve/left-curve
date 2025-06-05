use std::collections::HashMap;

use grug::{Denom, Int128, Number, NumberConst, Udec128, Uint128, Unsigned};

use super::{PerpsMarketParams, PerpsMarketState, Pnl};

/// The state of the perps vault
#[grug::derive(Serde, Borsh)]
pub struct PerpsVaultState {
    /// The denom that is deposited into the vault.
    pub denom: Denom,
    /// The amount of the denom that is deposited into the vault.
    pub deposits: Uint128,
    /// The amount of shares that that have been minted.
    pub shares: Uint128,
    /// The realised pnl of the vault.
    pub realised_pnl: Pnl,
}

impl PerpsVaultState {
    /// Returns the vault's PnL.
    ///
    /// This is the sum of the realised cash flow and the unrealized PnL capped
    /// at 0 for each market.
    pub fn vault_pnl(
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
                market.market_pnl(params, oracle_price)
            })
            .try_fold(Int128::ZERO, |acc, x| {
                Ok::<_, anyhow::Error>(acc.checked_add(x?)?)
            })?)
    }

    /// Returns the vault's withdrawable value.
    ///
    /// This is the sum of the deposits and the vault's PnL.
    pub fn withdrawable_value(
        &self,
        markets: &[PerpsMarketState],
        params: &HashMap<Denom, PerpsMarketParams>,
        oracle_prices: &HashMap<Denom, Udec128>,
    ) -> anyhow::Result<Int128> {
        let pnl = self.vault_pnl(markets, params, oracle_prices)?;
        Ok(pnl.checked_add(self.deposits.checked_into_signed()?)?)
    }
}
