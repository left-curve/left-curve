use grug::Udec128;

/// The response type for a margin account's `Health` query.
#[grug::derive(Serde)]
pub struct HealthResponse {
    /// The margin account's utilization rate.
    pub utilization_rate: Udec128,
    /// The total value of the margin account's debt.
    pub total_debt_value: Udec128,
    /// The total value of the margin account's collateral, adjusted for
    /// the collateral power of each denom.
    pub total_adjusted_collateral_value: Udec128,
}

/// Query messages for the margin account
#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the account's current sequence number.
    #[returns(u32)]
    Sequence {},
    /// Queries the health of the margin account.
    #[returns(HealthResponse)]
    Health {},
}
