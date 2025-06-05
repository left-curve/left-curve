use grug::{Int128, MathError, Number, Sign};

/// PnL for either a per market, a perp position, or the whole vault.
/// All units are in the vault’s deposit denom.
#[grug::derive(Serde, Borsh)]
#[derive(Default)]
pub struct Pnl {
    pub fees: Int128,
    pub price_pnl: Int128,
    pub funding_pnl: Int128,
}

impl Pnl {
    /// Returns the total realised cash flow.
    pub fn total(&self) -> Result<Int128, MathError> {
        Ok(self
            .fees
            .checked_add(self.price_pnl)?
            .checked_add(self.funding_pnl)?)
    }

    /// Add two realised cash flows together.
    pub fn add(&self, other: &Pnl) -> Result<Pnl, MathError> {
        Ok(Pnl {
            fees: self.fees.checked_add(other.fees)?,
            price_pnl: self.price_pnl.checked_add(other.price_pnl)?,
            funding_pnl: self.funding_pnl.checked_add(other.funding_pnl)?,
        })
    }

    /// Negates the PnL. Used to convert a position's PnL to the market's PnL.
    /// Negative PnL for a position means a positive PnL for the market/vault,
    /// and vice versa.
    pub fn checked_neg(&self) -> Result<Pnl, MathError> {
        Ok(Pnl {
            fees: self.fees.checked_neg()?,
            price_pnl: self.price_pnl.checked_neg()?,
            funding_pnl: self.funding_pnl.checked_neg()?,
        })
    }
}
