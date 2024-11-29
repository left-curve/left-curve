use {
    grug::{
        Addr, Coin, Duration, MathResult, MultiplyFraction, Number, NumberConst, Timestamp,
        Udec128, Uint128, Undefined,
    },
    std::{cmp::min, collections::BTreeMap},
};

pub type PositionIndex = u32;

pub type ClaimablePosition = Position<Uint128>;

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub unlocking_cliff: Duration,
    pub unlocking_vesting: Duration,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Create a vesting position for a user with the given schedule.
    /// Sender must attach a single coin.
    Create {
        user: Addr,
        schedule: Schedule,
    },
    // Terminate the vesting position.
    // When terminated, the snapshot of the vested so far is taken and stored
    Terminate {
        idx: PositionIndex,
    },
    /// Claim the withdrawable amount from the vesting position.
    Claim {
        idx: PositionIndex,
    },
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query a single vesting position by index.
    #[returns(ClaimablePosition)]
    Position { idx: PositionIndex },
    /// Enumerate all vesting positions.
    #[returns(BTreeMap<PositionIndex, ClaimablePosition>)]
    Positions {
        start_after: Option<PositionIndex>,
        limit: Option<u32>,
    },
    /// Enumerate all vesting positions belonging to a given user.
    #[returns(BTreeMap<PositionIndex, ClaimablePosition>)]
    PositionsByUser {
        user: Addr,
        start_after: Option<PositionIndex>,
        limit: Option<u32>,
    },
}

#[grug::derive(Serde, Borsh)]
pub struct Config {
    pub unlocking_schedule: Schedule,
}

#[grug::derive(Serde, Borsh)]
pub struct Schedule {
    pub start_time: Timestamp,
    pub cliff: Duration,
    pub vesting: Duration,
}

impl Schedule {
    pub fn compute_claimable_amount(
        &self,
        now: Timestamp,
        vesting_amount: Uint128,
    ) -> MathResult<Uint128> {
        let claim_percent = if now < self.start_time + self.cliff {
            // Before the cliff, no token is vested/unlocked.
            Udec128::ZERO
        } else if now < self.start_time + self.vesting {
            // After the cliff but before the period finishes, tokens vest/unlock
            // linearly through time.
            Udec128::checked_from_ratio(
                (now - self.start_time).into_nanos(),
                self.vesting.into_nanos(),
            )?
        } else {
            // After the period, all tokens are vested/unlocked.
            Udec128::ONE
        };

        vesting_amount.checked_mul_dec_floor(claim_percent)
    }
}

#[grug::derive(Serde, Borsh)]
pub struct Position<C = Undefined> {
    pub user: Addr,
    pub vesting_status: VestingStatus,
    pub vested_token: Coin,
    pub claimed_amount: Uint128,
    pub claimable_amount: C,
}

impl Position {
    pub fn new(user: Addr, vesting_schedule: Schedule, amount: Coin) -> Self {
        Self {
            user,
            vesting_status: VestingStatus::Active(vesting_schedule),
            vested_token: amount,
            claimed_amount: Uint128::ZERO,
            claimable_amount: Undefined::new(),
        }
    }

    pub fn with_claimable_amount(
        self,
        now: Timestamp,
        unlocking_schedule: &Schedule,
    ) -> Position<Uint128> {
        let claimable_amount = self
            .compute_claimable_amount(now, unlocking_schedule)
            .unwrap_or_default();

        Position {
            user: self.user,
            vesting_status: self.vesting_status,
            vested_token: self.vested_token,
            claimed_amount: self.claimed_amount,
            claimable_amount,
        }
    }
}

impl<T> Position<T> {
    pub fn compute_claimable_amount(
        &self,
        now: Timestamp,
        unlocking_schedule: &Schedule,
    ) -> anyhow::Result<Uint128> {
        // The claimable amount is the minimum between the claimable amount
        // from the vesting status and the unlocking schedule
        let claimable_amount = min(
            self.vesting_status
                .compute_claimable_amount(now, self.vested_token.amount)?,
            unlocking_schedule.compute_claimable_amount(now, self.vested_token.amount)?,
        );

        Ok(claimable_amount
            .checked_sub(self.claimed_amount)
            .unwrap_or_default())
    }

    pub fn full_claimed(&self) -> bool {
        match &self.vesting_status {
            VestingStatus::Active(_) => self.vested_token.amount == self.claimed_amount,
            VestingStatus::Terminated(terminated_amount) => {
                *terminated_amount == self.claimed_amount
            },
        }
    }
}

#[grug::derive(Serde, Borsh)]
pub enum VestingStatus {
    // Position is active. When active, the token stil being distributed
    Active(Schedule),
    // Position is terminated.
    // When terminated, the snapshot of the vested so far is taken and stored
    Terminated(Uint128),
}

impl VestingStatus {
    pub fn compute_claimable_amount(
        &self,
        now: Timestamp,
        vesting_amount: Uint128,
    ) -> MathResult<Uint128> {
        match self {
            VestingStatus::Active(schedule) => {
                schedule.compute_claimable_amount(now, vesting_amount)
            },
            VestingStatus::Terminated(amount) => Ok(*amount),
        }
    }
}
