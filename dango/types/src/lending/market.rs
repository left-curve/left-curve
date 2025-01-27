use grug::{Decimal, MultiplyFraction, Number, NumberConst, Timestamp, Udec128, Uint128};

use super::InterestRateModel;

const SECONDS_PER_YEAR: u32 = 31536000;

/// Configurations and state of a market.
#[grug::derive(Serde, Borsh)]
pub struct Market {
    /// The current interest rate model of this market.
    pub interest_rate_model: InterestRateModel,
    /// The total amount of coins borrowed from this market.
    pub total_borrowed: Uint128,
    /// The total amount of coins supplied to this market.
    pub total_supplied: Uint128,
    /// The current borrow index of this market. This is used to calculate the
    /// interest accrued on borrows.
    pub borrow_index: Udec128,
    /// The current supply index of this market. This is used to calculate the
    /// interest accrued on deposits.
    pub supply_index: Udec128,
    /// The last time the indices were updated.
    pub last_update_time: Timestamp,
}

impl Market {
    /// Computes the utilization rate of this market.
    pub fn utilization_rate(&self) -> anyhow::Result<Udec128> {
        if self.total_supplied == Uint128::ZERO {
            return Ok(Udec128::ZERO);
        }

        Ok(Udec128::checked_from_ratio(
            self.total_borrowed,
            self.total_supplied,
        )?)
    }

    /// Immutably updates the indices of this market and returns the new market
    /// state.
    pub fn update_indices(&self, current_time: Timestamp) -> anyhow::Result<Self> {
        // If there is no supply, then there is no interest to accrue
        if self.total_supplied == Uint128::ZERO {
            return Ok(self.clone());
        }

        // Calculate interest rates
        let utilization_rate = self.utilization_rate()?;
        let rates = self.interest_rate_model.calculate_rates(utilization_rate)?;

        // Update the indices
        let time_delta = current_time - self.last_update_time;
        let time_out_of_year =
            Udec128::checked_from_ratio(time_delta.into_seconds(), SECONDS_PER_YEAR as u128)?;
        let borrow_index = self.borrow_index.checked_mul(
            Udec128::ONE.checked_add(rates.borrow_rate.checked_mul(time_out_of_year)?)?,
        )?;
        let supply_index = self.supply_index.checked_mul(
            Udec128::ONE.checked_add(rates.deposit_rate.checked_mul(time_out_of_year)?)?,
        )?;

        // Update the total borrowed and supplied to account for interest
        let total_borrowed = self.total_borrowed.checked_mul_dec_ceil(borrow_index)?;
        let total_supplied = self.total_supplied.checked_mul_dec_ceil(supply_index)?;

        // Return the new market state
        Ok(Self {
            interest_rate_model: self.interest_rate_model.clone(),
            total_borrowed,
            total_supplied,
            borrow_index,
            supply_index,
            last_update_time: current_time,
        })
    }

    /// Immutably adds the given amount to the total supplied and returns the new
    /// market state.
    pub fn add_supplied(&self, amount: Uint128) -> anyhow::Result<Self> {
        Ok(Self {
            total_supplied: self.total_supplied.checked_add(amount)?,
            ..self.clone()
        })
    }

    /// Immutably deducts the given amount from the total supplied and returns
    /// the new market state.
    pub fn deduct_supplied(&self, amount: Uint128) -> anyhow::Result<Self> {
        Ok(Self {
            total_supplied: self.total_supplied.checked_sub(amount)?,
            ..self.clone()
        })
    }

    /// Immutably adds the given amount to the total borrowed and returns the
    /// new market state.
    pub fn add_borrowed(&self, amount: Uint128) -> anyhow::Result<Self> {
        Ok(Self {
            total_borrowed: self.total_borrowed.checked_add(amount)?,
            ..self.clone()
        })
    }

    /// Immutably deducts the given amount from the total borrowed and returns
    /// the new market state.
    pub fn deduct_borrowed(&self, amount: Uint128) -> anyhow::Result<Self> {
        Ok(Self {
            total_borrowed: self.total_borrowed.checked_sub(amount)?,
            ..self.clone()
        })
    }

    /// Calculates the actual debt for the given scaled amount. Makes sure to
    /// round up in favor of the protocol.
    pub fn calculate_debt(&self, scaled_amount: Udec128) -> anyhow::Result<Uint128> {
        Ok(scaled_amount
            .checked_mul(self.borrow_index)?
            .checked_ceil()?
            .into_int())
    }
}

/// A set of updates to be applied to a market.
#[grug::derive(Serde)]
pub struct MarketUpdates {
    pub interest_rate_model: Option<InterestRateModel>,
}
