use std::fmt::Display;

use tracing::warn;

use crate::{AppError, AppResult, Shared};

pub type SharedGasTracker = Shared<GasTracker>;

pub enum GasTracker {
    Limitless { used: u64 },
    Limited { limit: u64, used: u64 },
}

impl GasTracker {
    pub fn used(&self) -> u64 {
        match self {
            GasTracker::Limitless { used } => *used,
            GasTracker::Limited { used, .. } => *used,
        }
    }

    pub fn consume(&mut self, consumed: u64) -> AppResult<()> {
        match self {
            GasTracker::Limitless { used } => {
                *used += consumed;
                Ok(())
            },
            GasTracker::Limited { limit, used } => {
                let total_consumed = *used + consumed;
                if total_consumed > *limit {
                    #[cfg(feature = "tracing")]
                    warn!("Out of gas: max: {}, consumed: {}", limit, total_consumed);

                    Err(AppError::OutOfGas {
                        max: *limit,
                        consumed: total_consumed,
                    })
                } else {
                    *used += consumed;

                    Ok(())
                }
            },
        }
    }
}

impl Display for GasTracker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GasTracker::Limitless { used } => write!(f, "Gas info: limitless, used: {}", used),
            GasTracker::Limited { limit, used } => {
                write!(f, "Gas info: limit: {limit}, used: {used}")
            },
        }
    }
}

impl SharedGasTracker {
    pub fn new_limitless() -> Self {
        Shared::new(GasTracker::Limitless { used: 0 })
    }

    pub fn new_limited(limit: u64) -> Self {
        Shared::new(GasTracker::Limited { limit, used: 0 })
    }
}

impl<T> From<Option<T>> for SharedGasTracker
where
    T: Into<u64>,
{
    fn from(value: Option<T>) -> Self {
        match value {
            Some(value) => SharedGasTracker::new_limited(value.into()),
            None => SharedGasTracker::new_limitless(),
        }
    }
}
#[derive(Default)]
pub struct GasResponse {
    pub gas_used: u64,
}
