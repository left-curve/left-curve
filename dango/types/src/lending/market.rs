use {
    crate::lending::{InterestRateModel, NAMESPACE, SUBNAMESPACE},
    grug::{
        Bounded, Decimal, Denom, IsZero, MathResult, MultiplyFraction, NextNumber, Number,
        NumberConst, PrevNumber, StdResult, Timestamp, Udec128, Udec256, Uint128,
        ZeroInclusiveOneInclusive,
    },
};

/// Seconds in a year, assuming 365 days.
pub const SECONDS_PER_YEAR: u128 = 31536000;

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
    /// The total amount of coins supplied from this market scaled by the
    /// supply index.
    pub total_supplied_scaled: Uint128,
    /// The current supply index of this market. This is used to calculate the
    /// interest accrued on deposits.
    pub supply_index: Udec128,
    /// The last time the indices were updated.
    pub last_update_time: Timestamp,
    /// The pending scaled protocol fee that can be minted.
    pub pending_protocol_fee_scaled: Uint128,
}

impl Market {
    pub fn new(
        underlying_denom: &Denom,
        interest_rate_model: InterestRateModel,
    ) -> StdResult<Self> {
        Ok(Self {
            supply_lp_denom: underlying_denom.prepend(&[&NAMESPACE, &SUBNAMESPACE])?,
            interest_rate_model,
            total_borrowed_scaled: Udec256::ZERO,
            borrow_index: Udec128::ONE,
            total_supplied_scaled: Uint128::ZERO,
            supply_index: Udec128::ONE,
            last_update_time: Timestamp::ZERO,
            pending_protocol_fee_scaled: Uint128::ZERO,
        })
    }

    /// Computes the utilization rate of this market.
    pub fn utilization_rate(&self) -> MathResult<Bounded<Udec128, ZeroInclusiveOneInclusive>> {
        let total_borrowed = self.total_borrowed()?;
        let total_supplied = self.total_supplied()?;

        if total_supplied.is_zero() {
            return Ok(Bounded::new_unchecked(Udec128::ZERO));
        }

        let utilization_rate = Udec128::checked_from_ratio(total_borrowed, total_supplied)?;

        // Limit utilization rate to 100%
        // This can happen if 100% of the supply is borrowed, which can then cause
        // borrowing to outgrow the supply due to interest accrual.
        if utilization_rate > Udec128::new_percent(100) {
            return Ok(Bounded::new_unchecked(Udec128::new_percent(100)));
        }

        Ok(Bounded::new_unchecked(utilization_rate))
    }

    /// Immutably updates the indices of this market and returns the new market
    /// state.
    pub fn update_indices(self, current_time: Timestamp) -> MathResult<Self> {
        debug_assert!(
            current_time >= self.last_update_time,
            "last update time is in the future! current time: {:?}, last update time: {:?}",
            current_time,
            self.last_update_time
        );

        // If there is no supply or borrow or last update time is equal to the
        // current time, then there is no interest to accrue
        if self.total_supplied_scaled.is_zero()
            || self.total_borrowed_scaled.is_zero()
            || current_time == self.last_update_time
        {
            return Ok(self.set_last_update_time(current_time));
        }

        // Calculate interest rates
        let utilization_rate = self.utilization_rate()?;
        let (borrow_rate, supply_rate) = self.interest_rate_model.calculate_rates(utilization_rate);

        // Update the indices
        let time_delta = current_time - self.last_update_time;
        let time_out_of_year =
            Udec128::checked_from_ratio(time_delta.into_seconds(), SECONDS_PER_YEAR)?;
        let borrow_index = self
            .borrow_index
            .checked_mul(Udec128::ONE.checked_add(borrow_rate.checked_mul(time_out_of_year)?)?)?;
        let supply_index = self
            .supply_index
            .checked_mul(Udec128::ONE.checked_add(supply_rate.checked_mul(time_out_of_year)?)?)?;

        // Calculate the protocol fee
        let previous_total_borrowed = self.total_borrowed()?;
        let new_market = self.set_borrow_index(borrow_index);
        let new_total_borrowed = new_market.total_borrowed()?;
        let borrow_interest = new_total_borrowed.checked_sub(previous_total_borrowed)?;
        let protocol_fee =
            borrow_interest.checked_mul_dec(*new_market.interest_rate_model.reserve_factor)?;
        let protocol_fee_scaled = protocol_fee.checked_div_dec_floor(supply_index)?;

        // Return the new market state
        new_market
            .set_supply_index(supply_index)
            .set_last_update_time(current_time)
            .add_pending_protocol_fee(protocol_fee_scaled)
    }

    pub fn add_supplied(self, amount_scaled: Uint128) -> MathResult<Self> {
        Ok(Self {
            total_supplied_scaled: self.total_supplied_scaled.checked_add(amount_scaled)?,
            ..self
        })
    }

    pub fn deduct_supplied(self, amount_scaled: Uint128) -> MathResult<Self> {
        Ok(Self {
            total_supplied_scaled: self.total_supplied_scaled.checked_sub(amount_scaled)?,
            ..self
        })
    }

    /// Immutably adds the given amount to the scaled total borrowed and returns
    /// the new market state.
    pub fn add_borrowed(self, amount_scaled: Udec256) -> MathResult<Self> {
        Ok(Self {
            total_borrowed_scaled: self.total_borrowed_scaled.checked_add(amount_scaled)?,
            ..self
        })
    }

    /// Immutably deducts the given amount from the scaled total borrowed and
    /// returns the new market state.
    pub fn deduct_borrowed(self, amount_scaled: Udec256) -> MathResult<Self> {
        Ok(Self {
            total_borrowed_scaled: self.total_borrowed_scaled.checked_sub(amount_scaled)?,
            ..self
        })
    }

    /// Immutably adds the given amount to the pending protocol fee and returns
    /// the new market state.
    pub fn add_pending_protocol_fee(self, amount_scaled: Uint128) -> MathResult<Self> {
        Ok(Self {
            pending_protocol_fee_scaled: self
                .pending_protocol_fee_scaled
                .checked_add(amount_scaled)?,
            ..self
        })
    }

    /// Resets the pending protocol fee to zero.
    pub fn reset_pending_protocol_fee(self) -> Self {
        Self {
            pending_protocol_fee_scaled: Uint128::ZERO,
            ..self
        }
    }

    /// Immutably sets the supply index to the given value and returns the new
    /// market state.
    pub fn set_supply_index(self, index: Udec128) -> Self {
        Self {
            supply_index: index,
            ..self
        }
    }

    /// Immutably sets the borrow index to the given value and returns the new
    /// market state.
    pub fn set_borrow_index(self, index: Udec128) -> Self {
        Self {
            borrow_index: index,
            ..self
        }
    }

    /// Immutably sets the last update time to the given value and returns the
    /// new market state.
    pub fn set_last_update_time(self, time: Timestamp) -> Self {
        Self {
            last_update_time: time,
            ..self
        }
    }

    /// Immutably sets the interest rate model to the given value and returns
    /// the new market state.
    pub fn set_interest_rate_model(self, interest_rate_model: InterestRateModel) -> Self {
        Self {
            interest_rate_model,
            ..self
        }
    }

    /// Calculates the actual debt for the given scaled amount. Makes sure to
    /// round up in favor of the protocol.
    pub fn calculate_debt(&self, scaled_amount: Udec256) -> MathResult<Uint128> {
        scaled_amount
            .checked_mul(self.borrow_index.into_next())?
            .checked_ceil()?
            .into_int()
            .checked_into_prev()
    }

    /// Returns the total amount of coins supplied to this market.
    pub fn total_supplied(&self) -> MathResult<Uint128> {
        self.total_supplied_scaled
            .checked_add(self.pending_protocol_fee_scaled)?
            .checked_mul_dec_floor(self.supply_index)
    }

    /// Returns the total amount of coins borrowed from this market.
    pub fn total_borrowed(&self) -> MathResult<Uint128> {
        self.total_borrowed_scaled
            .checked_mul(self.borrow_index.into_next())?
            .checked_ceil()?
            .into_int()
            .checked_into_prev()
    }
}
