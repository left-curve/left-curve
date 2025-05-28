use grug::Int128;

/// Realised money movements for one perp market or the whole vault.
/// Positive values increase vault equity; negative decrease it.
/// All units are in the vault’s base denom.
#[grug::derive(Serde, Borsh)]
#[derive(Default)]
pub struct RealisedCashFlow {
    pub opening_fee: Int128,
    pub closing_fee: Int128,
    pub price_pnl: Int128,
    pub accrued_funding: Int128,
}
