use std::cmp::max;

use grug::{
    Dec128, Denom, Inner, Int128, MathError, Number, NumberConst, Sign, Timestamp, Udec128,
    Uint128, Unsigned,
};

use super::{PerpsMarketParams, PerpsPosition, RealisedCashFlow};

/// Current state of a perps market.
#[grug::derive(Serde, Borsh)]
pub struct PerpsMarketState {
    /// The denom of the market.
    pub denom: Denom,
    /// The long open interest of the market, in market denom units.
    pub long_oi: Uint128,
    /// The short open interest of the market, in market denom units.
    pub short_oi: Uint128,
    /// The last time the market was updated.
    pub last_updated: Timestamp,
    /// The latest funding rate of the market as a daily rate.
    pub last_funding_rate: Dec128,
    /// Cumulative funding that has accrued so far, **in vault-denom
    /// per 1 base asset**.  Every position stores the value that was
    /// current at its last modification (`entry_funding_index`) so
    /// that funding-PnL = q · (global_index − entry_index).
    pub last_funding_index: Dec128,
    /// The perps market accumulators. Used to calculate the NAV of the vault.
    pub accumulators: PerpsMarketAccumulators,
    /// The realised cash flow of the market.
    pub realised_cash_flow: RealisedCashFlow,
}

/// Global, per-market accumulators — enable O(1) NAV calculation.
#[grug::derive(Serde, Borsh)]
pub struct PerpsMarketAccumulators {
    /// Σ q_k * entry_execution_price_k  (signed, vault-denom units)
    pub cost_basis_sum: Dec128,

    /// Σ q_k * entry_funding_index_k  (signed, vault-denom units)
    pub funding_basis_sum: Dec128,

    /// Σ q_k * |q_k|  (signed, base-asset units)
    pub quadratic_fee_basis: Int128,
}

impl PerpsMarketAccumulators {
    pub fn new() -> Self {
        Self {
            cost_basis_sum: Dec128::ZERO,
            funding_basis_sum: Dec128::ZERO,
            quadratic_fee_basis: Int128::ZERO,
        }
    }

    /// Decrease the accumulators by the given position.
    pub fn decrease(&mut self, position: &PerpsPosition) -> Result<(), MathError> {
        self.cost_basis_sum = self.cost_basis_sum.checked_sub(
            position
                .size
                .checked_into_dec()?
                .checked_mul(position.entry_execution_price)?,
        )?;
        self.funding_basis_sum = self.funding_basis_sum.checked_sub(
            position
                .size
                .checked_into_dec()?
                .checked_mul(position.entry_funding_index)?,
        )?;
        self.quadratic_fee_basis = self
            .quadratic_fee_basis
            .checked_sub(position.size.checked_mul(position.size.checked_abs()?)?)?;

        Ok(())
    }

    /// Increase the accumulators by the given position.
    pub fn increase(&mut self, position: &PerpsPosition) -> Result<(), MathError> {
        self.cost_basis_sum = self.cost_basis_sum.checked_add(
            position
                .size
                .checked_into_dec()?
                .checked_mul(position.entry_execution_price)?,
        )?;
        self.funding_basis_sum = self.funding_basis_sum.checked_add(
            position
                .size
                .checked_into_dec()?
                .checked_mul(position.entry_funding_index)?,
        )?;
        self.quadratic_fee_basis = self
            .quadratic_fee_basis
            .checked_add(position.size.checked_mul(position.size.checked_abs()?)?)?;

        Ok(())
    }
}

impl PerpsMarketState {
    pub fn skew(&self) -> Result<Int128, MathError> {
        self.long_oi
            .checked_into_signed()?
            .checked_sub(self.short_oi.checked_into_signed()?)
    }

    /// Returns the pSkew = skew / skewScale capping the pSkew between [-1, 1].
    pub fn proportional_skew(&self, skew_scale: Uint128) -> Result<Dec128, MathError> {
        Ok(Dec128::checked_from_ratio(
            self.skew()?,
            skew_scale.checked_into_signed()?.into_inner(),
        )?
        .clamp(-Dec128::ONE, Dec128::ONE))
    }

    /// Returns the unrealized price PnL of the market.
    ///
    /// sum_of_traders_price_pnl = skew * oracle_price - cost_basis_sum + oracle_price / skew_scale * (skew^2 - quadratic_fee_basis / 2)
    /// market_pnl = -sum_of_traders_price_pnl
    pub fn unrealized_price_pnl(
        &self,
        oracle_price: Udec128,
        skew_scale: Uint128,
    ) -> Result<Int128, MathError> {
        let oracle_price = oracle_price.checked_into_signed()?;
        let skew = self.skew()?.checked_into_dec()?;
        let skew_scale = skew_scale.checked_into_dec()?.checked_into_signed()?;
        let cost_basis_sum = self.accumulators.cost_basis_sum;
        let quadratic_fee_basis = self.accumulators.quadratic_fee_basis.checked_into_dec()?;

        let last_term = oracle_price.checked_div(skew_scale)?.checked_mul(
            skew.checked_pow(2)?
                .checked_sub(quadratic_fee_basis.checked_div(Dec128::new(2))?)?,
        )?;
        let trader_price_pnl = skew
            .checked_mul(oracle_price)?
            .checked_sub(cost_basis_sum)?
            .checked_add(last_term)?;

        Ok(trader_price_pnl.into_int().checked_neg()?)
    }

    /// Returns the unrealized funding PnL of the market.
    ///
    /// sum_of_traders_funding_pnl = skew * funding_index - funding_basis_sum
    /// market_pnl = -sum_of_traders_funding_pnl
    pub fn unrealized_funding_pnl(&self) -> Result<Int128, MathError> {
        let funding_index = self.last_funding_index;
        let skew = self.skew()?.checked_into_dec()?;

        let traders_funding_pnl = skew
            .checked_mul(funding_index)?
            .checked_sub(self.accumulators.funding_basis_sum)?;

        Ok(traders_funding_pnl.into_int().checked_neg()?)
    }

    /// Returns the net asset value of the vault for this market.
    pub fn net_asset_value(
        &self,
        params: &PerpsMarketParams,
        oracle_price: Udec128,
    ) -> anyhow::Result<Int128> {
        // For the purpose of calculating the withdrawable vault value, we cap
        // the vault's unrealized PnL at 0, so that only losses for the vault
        // are considered. This way, future unrealized gains for the vault are
        // not counted so that we don't pay out gains that might not realize.
        // We also don't consider unrealized closing fees as these are always
        // gains for the vault.
        let price_pnl = max(
            Int128::ZERO,
            self.unrealized_price_pnl(oracle_price, params.skew_scale)?,
        );
        let funding_pnl = max(Int128::ZERO, self.unrealized_funding_pnl()?);

        Ok(self
            .realised_cash_flow
            .total()?
            .checked_add(price_pnl)?
            .checked_add(funding_pnl)?)
    }
}
