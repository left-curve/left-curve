use {
    anyhow::ensure,
    grug::{
        Addr, Coin, Duration, MultiplyFraction, NumberConst, Timestamp, Udec128, Uint128, Undefined,
    },
    std::collections::BTreeMap,
};

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
        idx: u64,
    },
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    #[returns(ClaimablePosition)]
    Position { idx: u64 },
    #[returns(BTreeMap<u64, ClaimablePosition>)]
    Positions {
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    #[returns(BTreeMap<u64, ClaimablePosition>)]
    PositionsByUser {
        user: Addr,
        start_after: Option<u64>,
        limit: Option<u32>,
    },
}

#[grug::derive(Serde, Borsh)]
pub struct Schedule<T = Timestamp> {
    pub start_time: T,
    pub cliff: Option<Duration>,
    pub vesting: Option<Duration>,
}

impl Schedule<Option<Timestamp>> {
    pub fn set_start_time(self, now: Timestamp) -> anyhow::Result<Schedule> {
        let start_time = if let Some(start_time) = self.start_time {
            ensure!(
                start_time >= now,
                "invalid start time: {:?}, now: {:?}",
                start_time,
                now
            );
            start_time
        } else {
            now
        };

        Ok(Schedule {
            start_time,
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
        ensure!(
            now >= self.schedule.start_time,
            "vesting has not started yet: start at: {:?}, now: {:?}",
            self.schedule.start_time,
            now
        );

        let mut time = self.schedule.start_time;

        if let Some(cliff) = self.schedule.cliff {
            time = time + cliff;

            ensure!(
                now >= time,
                "nothing to claim during cliff phase: end: {:?}, now: {:?}",
                time,
                now
            );
        }

        let claim_percent = if let Some(vesting) = self.schedule.vesting {
            let vesting_end = time + vesting;

            if now >= vesting_end {
                Udec128::ONE
            } else {
                Udec128::checked_from_ratio((now - time).into_nanos(), vesting.into_nanos())?
            }
        } else {
            Udec128::ONE
        };

        Ok(self.amount.amount.checked_mul_dec_floor(claim_percent)? - self.claimed_amount)
    }
}
