use {
    crate::lending::{InterestRateModel, NAMESPACE, SUBNAMESPACE},
    grug::{
        Decimal, Denom, MathResult, MultiplyFraction, NextNumber, Number, NumberConst, PrevNumber,
        QuerierExt, QuerierWrapper, StdResult, Timestamp, Udec128, Udec256, Uint128,
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
            supply_index: Udec128::ONE,
            last_update_time: Timestamp::ZERO,
            pending_protocol_fee_scaled: Uint128::ZERO,
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

    /// Returns the total amount of coins supplied to this market.
    pub fn total_supplied(&self, querier: QuerierWrapper) -> anyhow::Result<Uint128> {
        Ok(querier
            .query_supply(self.supply_lp_denom.clone())?
            .checked_add(self.pending_protocol_fee_scaled)?
            .checked_mul_dec(self.supply_index)?)
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
