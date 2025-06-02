use grug::{Dec128, Denom, Int128, Udec128};

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
    pub realized_pnl: Int128,
}
