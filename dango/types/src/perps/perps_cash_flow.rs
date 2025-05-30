use grug::{Int128, MathError, Number};

/// Realised money movements for one perp market or the whole vault.
/// Positive values increase vault equity; negative decrease it.
/// All units are in the vault’s deposit denom.
#[grug::derive(Serde, Borsh)]
#[derive(Default)]
pub struct RealisedCashFlow {
    pub opening_fee: Int128,
    pub closing_fee: Int128,
    pub price_pnl: Int128,
    pub accrued_funding: Int128,
}

impl RealisedCashFlow {
    /// Returns the total realised cash flow.
    pub fn total(&self) -> Result<Int128, MathError> {
        Ok(self
            .opening_fee
            .checked_add(self.closing_fee)?
            .checked_add(self.price_pnl)?
            .checked_add(self.accrued_funding)?)
    }

    /// Add two realised cash flows together.
    pub fn add(&self, other: &RealisedCashFlow) -> Result<RealisedCashFlow, MathError> {
        Ok(RealisedCashFlow {
            opening_fee: self.opening_fee.checked_add(other.opening_fee)?,
            closing_fee: self.closing_fee.checked_add(other.closing_fee)?,
            price_pnl: self.price_pnl.checked_add(other.price_pnl)?,
            accrued_funding: self.accrued_funding.checked_add(other.accrued_funding)?,
        })
    }
}
