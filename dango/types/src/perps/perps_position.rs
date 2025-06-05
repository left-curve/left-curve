use grug::{Dec128, Denom, Int128, MultiplyFraction, Number, Sign, Udec128, Unsigned};

use crate::oracle::PrecisionedPrice;

use super::{PerpsMarketParams, PerpsMarketState, Pnl};

/// The state of a perps position.
#[grug::derive(Serde, Borsh)]
pub struct PerpsPosition {
    /// The denom of the position.
    pub denom: Denom,
    /// The size of the position, denominated in the Market's Denom.
    pub size: Int128,
    /// The entry price of the position.
    pub entry_price: Udec128,
    /// The entry execution price of the position.
    pub entry_execution_price: Dec128,
    /// The skew at the time of entry.
    pub entry_skew: Int128,
    /// The funding index at the time of entry.
    pub entry_funding_index: Dec128,
    /// The realized pnl of the position.
    pub realized_pnl: Pnl,
}

impl PerpsPosition {
    pub fn unrealized_pnl(
        &self,
        order_size: Option<Int128>,
        fill_price: Dec128,
        vault_denom_price: &PrecisionedPrice,
        market_state: &PerpsMarketState,
        market_params: &PerpsMarketParams,
    ) -> anyhow::Result<Pnl> {
        let order_size = order_size.unwrap_or(self.size.checked_neg()?);

        // TODO: should round away from zero?
        let realised_price_pnl = self.size.checked_mul_dec(
            fill_price
                .checked_sub(self.entry_execution_price)?
                .checked_div(vault_denom_price.unit_price()?.checked_into_signed()?)?,
        )?;
        let realised_funding_pnl = self.size.checked_mul_dec(
            market_state
                .last_funding_index
                .checked_sub(self.entry_funding_index)?,
        )?;

        let fee_usd = market_state.calculate_order_fee(
            order_size,
            fill_price,
            market_params.maker_fee,
            market_params.taker_fee,
        )?;
        let fee_in_vault_denom = vault_denom_price.unit_amount_from_value(fee_usd)?;

        Ok(Pnl {
            price_pnl: realised_price_pnl,
            funding_pnl: realised_funding_pnl,
            fees: fee_in_vault_denom.checked_into_signed()?.checked_neg()?,
        })
    }
}

/// The response when querying perps positions
#[grug::derive(Serde, Borsh)]
pub struct PerpsPositionResponse {
    /// The denom of the position.
    pub denom: Denom,
    /// The size of the position, denominated in the Market's Denom.
    pub size: Int128,
    /// The entry price of the position.
    pub entry_price: Udec128,
    /// The entry execution price of the position.
    pub entry_execution_price: Dec128,
    /// The skew at the time of entry.
    pub entry_skew: Int128,
    /// The funding index at the time of entry.
    pub entry_funding_index: Dec128,
    /// The realized pnl of the position.
    pub realized_pnl: Pnl,
    /// The unrealized pnl of the position, if the whole position was closed now.
    pub unrealized_pnl: Pnl,
}
