use std::cmp::max;

use grug::{
    Bounded, Dec128, Denom, Inner, Int128, IsZero, MathError, MultiplyFraction, Number,
    NumberConst, Sign, Signed, Timestamp, Udec128, Uint128, Unsigned, ZeroInclusiveOneExclusive,
};

use crate::{oracle::PrecisionedPrice, perps::same_side};

use super::{PerpsMarketParams, PerpsPosition, Pnl};

pub const NANOSECONDS_PER_DAY: u128 = 86_400_000_000_000;

/// The maximum funding rate. Set to 96% per day.
pub const MAX_FUNDING_RATE: Dec128 = Dec128::new_percent(96);

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
    /// The latest funding rate of the market as a daily rate. A positive rate
    /// means that longs pay funding to shorts.
    pub last_funding_rate: Dec128,
    /// Cumulative funding that has accrued so far, **in vault-denom
    /// per 1 base asset**.  Every position stores the value that was
    /// current at its last modification (`entry_funding_index`) so
    /// that funding-PnL = q · (global_index − entry_index).
    pub last_funding_index: Dec128,
    /// The perps market accumulators. Used to calculate the NAV of the vault.
    pub accumulators: PerpsMarketAccumulators,
    /// The realised pnl of the market.
    pub realised_pnl: Pnl,
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
    /// Returns the skew of the market (long_oi - short_oi).
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

    /// Calculate the fill price for an order.
    pub fn calculate_fill_price(
        &self,
        order_size: Int128,
        oracle_unit_price: Dec128,
        skew_scale: Uint128,
    ) -> Result<Dec128, MathError> {
        let skew = self.skew()?;
        let skew_scale = skew_scale.checked_into_signed()?;

        let pd_before = Dec128::checked_from_ratio(skew, skew_scale)?;
        let pd_after = Dec128::checked_from_ratio(skew.checked_add(order_size)?, skew_scale)?;

        let price_before =
            oracle_unit_price.checked_add(oracle_unit_price.checked_mul(pd_before)?)?;
        let price_after =
            oracle_unit_price.checked_add(oracle_unit_price.checked_mul(pd_after)?)?;

        let fill_price = price_before
            .checked_add(price_after)?
            .checked_div(Dec128::new(2))?;

        Ok(fill_price)
    }

    /// Calculate the fee in USD for an order.
    ///
    /// Arguments:
    /// - `order_size`: The size of the order.
    /// - `fill_price`: The price at which the order is filled.
    /// - `maker_fee_rate`: The maker fee rate.
    /// - `taker_fee_rate`: The taker fee rate.
    ///
    /// Returns the fee in USD.
    pub fn calculate_order_fee(
        &self,
        order_size: Int128,
        fill_price: Dec128,
        maker_fee_rate: Bounded<Udec128, ZeroInclusiveOneExclusive>,
        taker_fee_rate: Bounded<Udec128, ZeroInclusiveOneExclusive>,
    ) -> Result<Udec128, MathError> {
        let skew = self.skew()?;
        let notional_diff = order_size.checked_mul_dec(fill_price)?;

        // Check if trade keeps skew on one side
        let new_skew = skew.checked_add(order_size)?;
        let fee_usd = if same_side(skew, new_skew) {
            // This trade keeps the skew on the same side.
            let fee_rate = if same_side(notional_diff, skew) {
                taker_fee_rate
            } else {
                maker_fee_rate
            };

            fee_rate
                .into_inner()
                .checked_mul(notional_diff.unsigned_abs().checked_into_dec()?)?
        } else {
            // This trade flips the skew. Apply maker fee on the portion that
            // decreases the skew towards zero, and taker fee on the portion that
            // increases the skew away from zero.
            let taker_portion =
                Dec128::checked_from_ratio(order_size.checked_add(skew)?, order_size)?
                    .unsigned_abs();
            let maker_portion = Udec128::ONE.checked_sub(taker_portion)?;
            let taker_fee = taker_portion
                .checked_mul(taker_fee_rate.into_inner())?
                .checked_mul(notional_diff.unsigned_abs().checked_into_dec()?)?;
            let maker_fee = maker_portion
                .checked_mul(maker_fee_rate.into_inner())?
                .checked_mul(notional_diff.unsigned_abs().checked_into_dec()?)?;

            taker_fee.checked_add(maker_fee)?
        };

        Ok(fee_usd)
    }

    /// Calculates the new funding rate and funding index and returns a new
    /// `PerpsMarketState` with these values.
    pub fn update_funding(
        &self,
        params: &PerpsMarketParams,
        timestamp: Timestamp,
        market_denom_price: &PrecisionedPrice,
        vault_denom_price: &PrecisionedPrice,
    ) -> anyhow::Result<Self> {
        // Update the funding rate
        let time_elapsed_days = Udec128::checked_from_ratio(
            timestamp
                .into_nanos()
                .checked_sub(self.last_updated.into_nanos())
                .ok_or_else(|| anyhow::anyhow!("time elapsed is negative"))?,
            NANOSECONDS_PER_DAY,
        )?
        .checked_into_signed()?;
        let proportional_skew = self.proportional_skew(params.skew_scale)?;
        let current_funding_velocity =
            proportional_skew.checked_mul(params.max_funding_velocity.checked_into_signed()?)?;
        let funding_rate = self
            .last_funding_rate
            .checked_add(current_funding_velocity.checked_mul(time_elapsed_days)?)?;
        let funding_rate = funding_rate.clamp(-MAX_FUNDING_RATE, MAX_FUNDING_RATE);

        // Update current funding index
        let average_funding_rate = self
            .last_funding_rate
            .checked_add(funding_rate)?
            .checked_div(Dec128::ONE + Dec128::ONE)?;
        let market_denom_price_in_vault_denom = market_denom_price
            .unit_price()?
            .checked_div(vault_denom_price.unit_price()?)?;
        let unrecorded_funding = average_funding_rate
            .checked_mul(time_elapsed_days)?
            .checked_mul(market_denom_price_in_vault_denom.checked_into_signed()?)?;
        let funding_index = self.last_funding_index.checked_sub(unrecorded_funding)?;

        Ok(Self {
            last_funding_rate: funding_rate,
            last_funding_index: funding_index,
            last_updated: timestamp,
            ..self.clone()
        })
    }

    /// Calculates the new open interest and returns a new `PerpsMarketState` with these values.
    pub fn update_open_interest(
        &self,
        current_pos: &PerpsPosition,
        new_pos: &PerpsPosition,
    ) -> Result<Self, MathError> {
        let mut long_oi = self.long_oi;
        let mut short_oi = self.short_oi;
        if current_pos.size.is_positive() {
            long_oi = long_oi.checked_sub(current_pos.size.unsigned_abs())?;
        } else {
            short_oi = short_oi.checked_sub(current_pos.size.unsigned_abs())?;
        }
        if new_pos.size.is_positive() {
            long_oi = long_oi.checked_add(new_pos.size.unsigned_abs())?;
        } else {
            short_oi = short_oi.checked_add(new_pos.size.unsigned_abs())?;
        }

        Ok(Self {
            long_oi,
            short_oi,
            ..self.clone()
        })
    }

    /// Updates the accumulators and returns a new `PerpsMarketState` with these values.
    pub fn update_accumulators(
        &self,
        current_pos: &PerpsPosition,
        new_pos: &PerpsPosition,
    ) -> Result<Self, MathError> {
        let mut accumulators = self.accumulators.clone();
        if current_pos.size.is_non_zero() {
            accumulators.decrease(&current_pos)?;
        }
        accumulators.increase(&new_pos)?;

        Ok(Self {
            accumulators,
            ..self.clone()
        })
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

    /// Returns the PnL for this market from the perspective of the vault.
    ///
    /// This is the sum of the realised cash flow and the unrealized PnL capped
    /// at 0.
    pub fn market_pnl(
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
            .realised_pnl
            .total()?
            .checked_add(price_pnl)?
            .checked_add(funding_pnl)?)
    }
}
