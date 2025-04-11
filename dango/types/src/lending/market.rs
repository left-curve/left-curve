use {
    crate::lending::InterestRateModel,
    anyhow::ensure,
    grug::{
        Decimal, Denom, IsZero, MultiplyFraction, NextNumber, Number, NumberConst, PrevNumber,
        Querier, QuerierExt, QuerierWrapper, StdError, Timestamp, Udec128, Udec256, Uint128,
    },
    std::error::Error,
};

/// Seconds in a year, assuming 365 days.
pub const SECONDS_PER_YEAR: u32 = 31536000;

/// Configurations and state of a market.
#[grug::derive(Serde, Borsh)]
pub struct Market {
    /// The LP token denom that is minted when coins are deposited on the supply
    /// side.
    pub supply_lp_denom: Denom,
    /// The current interest rate model of this market.
    pub interest_rate_model: InterestRateModel,
    /// The total amount of coins borrowed from this market scaled by the
    /// borrow index.
    pub total_borrowed_scaled: Udec256,
    /// The current borrow index of this market. This is used to calculate the
    /// interest accrued on borrows.
    pub borrow_index: Udec128,
    /// The current supply index of this market. This is used to calculate the
    /// interest accrued on deposits.
    pub supply_index: Udec128,
    /// The last time the indices were updated.
    pub last_update_time: Timestamp,
    /// The pending scaled protocol fee that can be minted.
    pub pending_protocol_fee_scaled: Uint128,
}

impl Market {
    pub fn new(supply_lp_denom: Denom, interest_rate_model: InterestRateModel) -> Self {
        Self {
            supply_lp_denom,
            interest_rate_model,
            total_borrowed_scaled: Udec256::ZERO,
            borrow_index: Udec128::ONE,
            supply_index: Udec128::ONE,
            last_update_time: Timestamp::ZERO,
            pending_protocol_fee_scaled: Uint128::ZERO,
        }
    }

    /// Computes the utilization rate of this market.
    pub fn utilization_rate<E>(&self, querier: &dyn Querier<Error = E>) -> anyhow::Result<Udec128>
    where
        E: From<StdError> + Error + Send + Sync + 'static,
    {
        let total_borrowed = self.total_borrowed()?;
        let total_supplied = self.total_supplied(querier)?;

        if total_supplied.is_zero() {
            return Ok(Udec128::ZERO);
        }

        let utilization_rate = Udec128::checked_from_ratio(total_borrowed, total_supplied)?;

        // Limit utilization rate to 100%
        // This can happen if 100% of the supply is borrowed, which can then cause
        // borrowing to outgrow the supply due to interest accrual.
        if utilization_rate > Udec128::new_percent(100) {
            return Ok(Udec128::new_percent(100));
        }

        Ok(utilization_rate)
    }

    /// Immutably updates the indices of this market and returns the new market
    /// state.
    pub fn update_indices<E>(
        &self,
        querier: &dyn Querier<Error = E>,
        current_time: Timestamp,
    ) -> anyhow::Result<Self>
    where
        E: From<StdError> + Error + Send + Sync + 'static,
    {
        ensure!(
            current_time >= self.last_update_time,
            "last update time is in the future"
        );

        // If there is no supply or borrow or last update time is equal to the
        // current time, then there is no interest to accrue
        if self.total_supplied(querier)?.is_zero()
            || self.total_borrowed_scaled.is_zero()
            || current_time == self.last_update_time
        {
            return Ok(self.set_last_update_time(current_time));
        }

        // Calculate interest rates
        let utilization_rate = self.utilization_rate(querier)?;
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

        // Calculate the protocol fee
        let previous_total_borrowed = self.total_borrowed()?;
        let new_market = self.set_borrow_index(borrow_index);
        let new_total_borrowed = new_market.total_borrowed()?;
        let borrow_interest = new_total_borrowed.checked_sub(previous_total_borrowed)?;
        let protocol_fee =
            borrow_interest.checked_mul_dec(self.interest_rate_model.reserve_factor())?;
        let protocol_fee_scaled = protocol_fee.checked_div_dec_floor(supply_index)?;

        // Return the new market state
        new_market
            .set_supply_index(supply_index)
            .set_last_update_time(current_time)
            .add_pending_protocol_fee(protocol_fee_scaled)
    }

    /// Immutably adds the given amount to the scaled total borrowed and returns
    /// the new market state.
    pub fn add_borrowed(&self, amount_scaled: Udec256) -> anyhow::Result<Self> {
        Ok(Self {
            total_borrowed_scaled: self.total_borrowed_scaled.checked_add(amount_scaled)?,
            ..self.clone()
        })
    }

    /// Immutably deducts the given amount from the scaled total borrowed and
    /// returns the new market state.
    pub fn deduct_borrowed(&self, amount_scaled: Udec256) -> anyhow::Result<Self> {
        Ok(Self {
            total_borrowed_scaled: self.total_borrowed_scaled.checked_sub(amount_scaled)?,
            ..self.clone()
        })
    }

    /// Immutably adds the given amount to the pending protocol fee and returns
    /// the new market state.
    pub fn add_pending_protocol_fee(&self, amount_scaled: Uint128) -> anyhow::Result<Self> {
        Ok(Self {
            pending_protocol_fee_scaled: self
                .pending_protocol_fee_scaled
                .checked_add(amount_scaled)?,
            ..self.clone()
        })
    }

    /// Resets the pending protocol fee to zero.
    pub fn reset_pending_protocol_fee(&self) -> Self {
        Self {
            pending_protocol_fee_scaled: Uint128::ZERO,
            ..self.clone()
        }
    }

    /// Immutably sets the supply index to the given value and returns the new
    /// market state.
    pub fn set_supply_index(&self, index: Udec128) -> Self {
        Self {
            supply_index: index,
            ..self.clone()
        }
    }

    /// Immutably sets the borrow index to the given value and returns the new
    /// market state.
    pub fn set_borrow_index(&self, index: Udec128) -> Self {
        Self {
            borrow_index: index,
            ..self.clone()
        }
    }

    /// Immutably sets the last update time to the given value and returns the
    /// new market state.
    pub fn set_last_update_time(&self, time: Timestamp) -> Self {
        Self {
            last_update_time: time,
            ..self.clone()
        }
    }

    /// Immutably sets the interest rate model to the given value and returns
    /// the new market state.
    pub fn set_interest_rate_model(&self, interest_rate_model: InterestRateModel) -> Self {
        Self {
            interest_rate_model,
            ..self.clone()
        }
    }

    /// Calculates the actual debt for the given scaled amount. Makes sure to
    /// round up in favor of the protocol.
    pub fn calculate_debt(&self, scaled_amount: Udec256) -> anyhow::Result<Uint128> {
        Ok(scaled_amount
            .checked_mul(self.borrow_index.into_next())?
            .checked_ceil()?
            .into_int()
            .checked_into_prev()?)
    }

    /// Returns the total amount of coins supplied to this market.
    pub fn total_supplied<E>(&self, querier: &dyn Querier<Error = E>) -> anyhow::Result<Uint128>
    where
        E: From<StdError> + Error + Send + Sync + 'static,
    {
        let wrapper = QuerierWrapper::new(querier);
        let total_lp_supply = wrapper.query_supply(self.supply_lp_denom.clone())?;
        let scaled_total_supply = total_lp_supply.checked_add(self.pending_protocol_fee_scaled)?;
        Ok(scaled_total_supply.checked_mul_dec(self.supply_index)?)
    }

    /// Returns the total amount of coins borrowed from this market.
    pub fn total_borrowed(&self) -> anyhow::Result<Uint128> {
        Ok(self
            .total_borrowed_scaled
            .checked_mul(self.borrow_index.into_next())?
            .checked_ceil()?
            .into_int()
            .checked_into_prev()?)
    }
}

/// A set of updates to be applied to a market.
#[grug::derive(Serde)]
pub struct MarketUpdates {
    pub interest_rate_model: Option<InterestRateModel>,
}
