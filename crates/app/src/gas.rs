use crate::{AppError, AppResult, Shared};

pub type SharedGasTracker = Shared<GasTracker>;

#[derive(Default)]
pub struct GasTracker {
    pub limit: u64,
    pub remaining: u64,
}

impl GasTracker {
    pub fn used(&self) -> u64 {
        self.limit - self.remaining
    }

    pub fn reset(&mut self, limit: u64) {
        self.remaining = limit;
        self.limit = limit;
    }

    pub fn reset_to_max(&mut self) {
        self.remaining = u64::MAX;
        self.limit = u64::MAX;
    }
}

impl GasTracker {
    pub fn deduct(&mut self, used: u64) -> AppResult<()> {
        if self.remaining < used {
            Err(AppError::OutOfGas {
                max: self.limit,
                consumed: self.limit + used - self.remaining,
            })
        } else {
            self.remaining -= used;

            Ok(())
        }
    }
}

impl SharedGasTracker {
    pub fn new_max() -> Self {
        Shared::new(GasTracker {
            limit: u64::MAX,
            remaining: u64::MAX,
        })
    }

    pub fn new_with_limit(limit: u64) -> Self {
        Shared::new(GasTracker {
            limit,
            remaining: limit,
        })
    }
}

#[derive(Default)]
pub struct GasResponse {
    pub gas_used: u64,
}
