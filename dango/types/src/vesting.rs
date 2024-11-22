use {
    grug::{
        Addr, Coin, Duration, MultiplyFraction, NumberConst, Timestamp, Udec128, Uint128, Undefined,
    },
    std::collections::BTreeMap,
};

pub type PositionIndex = u32;

pub type ClaimablePosition = Position<Uint128>;

#[grug::derive(Serde)]
pub struct InstantiateMsg {}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    CreatePosition {
        user: Addr,
        schedule: Schedule<Option<Timestamp>>,
    },
    Claim {
        idx: PositionIndex,
    },
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    #[returns(ClaimablePosition)]
    Position { idx: PositionIndex },
    #[returns(BTreeMap<PositionIndex, ClaimablePosition>)]
    Positions {
        start_after: Option<PositionIndex>,
        limit: Option<u32>,
    },
    #[returns(BTreeMap<PositionIndex, ClaimablePosition>)]
    PositionsByUser {
        user: Addr,
        start_after: Option<PositionIndex>,
        limit: Option<u32>,
    },
}

#[grug::derive(Serde, Borsh)]
pub struct Schedule<T = Timestamp> {
    pub start_time: T,
    pub cliff: Duration,
    pub vesting: Duration,
}

impl Schedule<Option<Timestamp>> {
    pub fn set_start_time(self, now: Timestamp) -> anyhow::Result<Schedule> {
        Ok(Schedule {
            start_time: self.start_time.unwrap_or(now),
            cliff: self.cliff,
            vesting: self.vesting,
        })
    }
}

#[grug::derive(Serde, Borsh)]
pub struct Position<C = Undefined> {
    pub user: Addr,
    pub schedule: Schedule,
    pub amount: Coin,
    pub claimed_amount: Uint128,
    pub claimable_amount: C,
}

impl Position {
    pub fn new(user: Addr, schedule: Schedule, amount: Coin) -> Self {
        Self {
            user,
            schedule,
            amount,
            claimed_amount: Uint128::ZERO,
            claimable_amount: Undefined::new(),
        }
    }

    pub fn with_claimable_amount(self, now: Timestamp) -> Position<Uint128> {
        let claimable_amount = self.compute_claimable_amount(now).unwrap_or_default();
        Position {
            user: self.user,
            schedule: self.schedule,
            amount: self.amount,
            claimed_amount: self.claimed_amount,
            claimable_amount,
        }
    }
}

impl<T> Position<T> {
    pub fn compute_claimable_amount(&self, now: Timestamp) -> anyhow::Result<Uint128> {
        let vesting_start = self.schedule.start_time + self.schedule.cliff;

        if vesting_start >= now {
            return Ok(Uint128::ZERO);
        }

        let claim_percent = if now >= self.schedule.vesting + vesting_start {
            Udec128::ONE
        } else {
            Udec128::checked_from_ratio(
                (now - vesting_start).into_nanos(),
                self.schedule.vesting.into_nanos(),
            )?
        };

        Ok(self.amount.amount.checked_mul_dec_floor(claim_percent)? - self.claimed_amount)
    }
}
