use grug::{
    Dec128, Denom, Inner, Int128, MathError, Number, NumberConst, Sign, Timestamp, Uint128,
    Unsigned,
};

use super::PerpsPosition;

/// Current state of a perps market.
#[grug::derive(Serde, Borsh)]
pub struct PerpsMarketState {
    /// The denom of the market.
    pub denom: Denom,
    /// The long open interest of the market.
    pub long_oi: Uint128,
    /// The short open interest of the market.
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
}

/// Global, per-market accumulators — enable O(1) NAV calculation.
#[grug::derive(Serde, Borsh)]
pub struct PerpsMarketAccumulators {
    /// Σ q_k  (signed)
    pub net_position_q: Int128,

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
            net_position_q: Int128::ZERO,
            cost_basis_sum: Dec128::ZERO,
            funding_basis_sum: Dec128::ZERO,
            quadratic_fee_basis: Int128::ZERO,
        }
    }

    /// Decrease the accumulators by the given position.
    pub fn decrease(&mut self, position: &PerpsPosition) -> Result<(), MathError> {
        self.net_position_q = self.net_position_q.checked_sub(position.size)?;
        self.cost_basis_sum = self.cost_basis_sum.checked_sub(
            position
                .size
                .checked_into_dec()?
                .checked_mul(position.entry_execution_price.checked_into_signed()?)?,
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
        self.net_position_q = self.net_position_q.checked_add(position.size)?;
        self.cost_basis_sum = self.cost_basis_sum.checked_add(
            position
                .size
                .checked_into_dec()?
                .checked_mul(position.entry_execution_price.checked_into_signed()?)?,
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
}
