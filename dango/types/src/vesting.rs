use {
    grug::{
        Addr, Duration, MathResult, MultiplyFraction, Number, NumberConst, Timestamp, Udec128,
        Uint128,
    },
    std::{cmp::min, collections::BTreeMap},
};

#[grug::derive(Serde, Borsh)]
pub struct Schedule {
    pub start_time: Timestamp,
    pub cliff: Duration,
    pub period: Duration,
}

impl Schedule {
    pub fn compute_claimable(&self, now: Timestamp, total: Uint128) -> MathResult<Uint128> {
        let claim_percent = if now < self.start_time + self.cliff {
            // Before the cliff, no token is vested/unlocked.
            Udec128::ZERO
        } else if now < self.start_time + self.period {
            // After the cliff but before the period finishes, tokens vest/unlock
            // linearly through time.
            Udec128::checked_from_ratio(
                (now - self.start_time).into_nanos(),
                self.period.into_nanos(),
            )?
        } else {
            // After the period, all tokens are vested/unlocked.
            Udec128::ONE
        };

        total.checked_mul_dec_floor(claim_percent)
    }
}

#[grug::derive(Serde, Borsh)]
pub enum VestingStatus {
    /// Position is actively being vested.
    Active(Schedule),
    /// Position has been terminated.
    ///
    /// The amount of tokens that have been vested at the time of termination is
    /// stored here.
    Terminated(Uint128),
}

impl VestingStatus {
    pub fn compute_claimable(&self, now: Timestamp, total: Uint128) -> MathResult<Uint128> {
        match self {
            VestingStatus::Active(schedule) => schedule.compute_claimable(now, total),
            VestingStatus::Terminated(vested) => Ok(*vested),
        }
    }
}

#[grug::derive(Serde, Borsh)]
pub struct Position {
    pub vesting_status: VestingStatus,
    pub total: Uint128,
    pub claimed: Uint128,
}

impl Position {
    pub fn compute_claimable(
        &self,
        now: Timestamp,
        unlocking_schedule: &Schedule,
    ) -> MathResult<Uint128> {
        // The claimable amount is the minimum between the claimable amount
        // from the vesting status and the unlocking schedule
        let claimable = min(
            self.vesting_status.compute_claimable(now, self.total)?,
            unlocking_schedule.compute_claimable(now, self.total)?,
        );

        Ok(claimable.checked_sub(self.claimed).unwrap_or_default())
    }
}

#[grug::derive(Serde)]
pub struct PositionResponse {
    pub position: Position,
    pub claimable: Uint128,
}

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub unlocking_cliff: Duration,
    pub unlocking_period: Duration,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Create a vesting position for a user with the given schedule.
    /// Sender must attach a non-zero amount of Dango token and nothing else.
    Create {
        user: Addr,
        schedule: Schedule,
    },
    // Terminate a user's vesting position.
    Terminate {
        user: Addr,
    },
    /// Claim the withdrawable amount from the vesting position.
    Claim {},
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query a single vesting position by index.
    #[returns(PositionResponse)]
    Position { user: Addr },
    /// Enumerate all vesting positions.
    #[returns(BTreeMap<Addr, PositionResponse>)]
    Positions {
        start_after: Option<Addr>,
        limit: Option<u32>,
    },
}
