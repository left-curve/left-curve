use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::{AppError, AppResult};

#[derive(Default)]
pub struct SharedGasTracker {
    inner: Arc<RwLock<GasTracker>>,
}

impl SharedGasTracker {
    pub fn write(&self) -> RwLockWriteGuard<'_, GasTracker> {
        self.inner
            .write()
            .unwrap_or_else(|err| panic!("poisoned lock: {err:?}"))
    }

    pub fn read(&self) -> RwLockReadGuard<'_, GasTracker> {
        self.inner
            .read()
            .unwrap_or_else(|err| panic!("poisoned lock: {err:?}"))
    }
}

impl Clone for SharedGasTracker {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

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

#[derive(Default)]
pub struct GasResponse {
    pub gas_used: u64,
}
