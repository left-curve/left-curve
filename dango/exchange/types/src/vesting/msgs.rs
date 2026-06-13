use {
    crate::vesting::{Position, Schedule},
    grug_math::Uint128,
    grug_types::{Addr, Duration},
    std::collections::BTreeMap,
};

#[grug_types::derive(Serde)]
pub struct PositionResponse {
    pub position: Position,
    pub claimable: Uint128,
}

#[grug_types::derive(Serde)]
pub struct InstantiateMsg {
    pub unlocking_cliff: Duration,
    pub unlocking_period: Duration,
}

#[grug_types::derive(Serde)]
pub enum ExecuteMsg {
    /// Create a vesting position for a user with the given schedule.
    ///
    /// Sender must be the chain owner, and attach a non-zero amount of Dango
    /// token and nothing else.
    Create { user: Addr, schedule: Schedule },
    /// Terminate a user's vesting position.
    ///
    /// Sender must be the chain owner.
    Terminate { user: Addr },
    /// Claim the withdrawable amount from the vesting position.
    ///
    /// Sender must have a non-zero amount of claimable tokens.
    Claim {},
}

#[grug_types::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query a single vesting position by user address.
    #[returns(PositionResponse)]
    Position { user: Addr },
    /// Enumerate all vesting positions.
    #[returns(BTreeMap<Addr, PositionResponse>)]
    Positions {
        start_after: Option<Addr>,
        limit: Option<u32>,
    },
}
