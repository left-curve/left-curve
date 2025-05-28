use grug::{Denom, Uint128};

use super::RealisedCashFlow;

/// The state of the perps vault
#[grug::derive(Serde, Borsh)]
pub struct PerpsVaultState {
    /// The denom that is deposited into the vault.
    pub denom: Denom,
    /// The amount of the denom that is deposited into the vault.
    pub deposits: Uint128,
    /// The amount of shares that that have been minted.
    pub shares: Uint128,
    /// The realised cash flow of the vault.
    pub realised_cash_flow: RealisedCashFlow,
}
