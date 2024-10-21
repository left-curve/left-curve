use {
    crate::Shared,
    grug_types::{StdError, StdResult},
    std::fmt::{self, Display},
};

/// Tracks gas consumption; throws error if gas limit is exceeded.
#[non_exhaustive]
pub struct GasTracker<T = GUnbound> {
    inner: Shared<T>,
}

impl<T> Clone for GasTracker<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl GasTracker {
    /// Create a new gas tracker, with or without a gas limit.
    pub fn new(maybe_limit: Option<u64>) -> GasTracker<GUnbound> {
        GasTracker {
            inner: Shared::new(GUnbound {
                limit: maybe_limit,
                used: 0,
            }),
        }
    }

    /// Create a new gas tracker without a gas limit.
    pub fn new_limitless() -> GasTracker<GLimitLess> {
        GasTracker {
            inner: Shared::new(GLimitLess { used: 0 }),
        }
    }

    /// Create a new gas tracker with the given gas limit.
    pub fn new_limited(limit: u64) -> GasTracker<GLimited> {
        GasTracker {
            inner: Shared::new(GLimited { limit, used: 0 }),
        }
    }
}

impl GasTracker<GUnbound> {
    /// Return the gas limit. `None` if there isn't a limit.
    ///
    /// Panics if lock is poisoned.
    pub fn limit(&self) -> Option<u64> {
        self.inner.read_access().limit
    }

    /// Return the amount of gas already used.
    ///
    /// Panics if lock is poisoned.
    pub fn used(&self) -> u64 {
        self.inner.read_access().used
    }

    /// Return the amount of gas remaining. `None` if there isn't a limit.
    ///
    /// Panics if lock is poisoned.
    pub fn remaining(&self) -> Option<u64> {
        self.inner.read_with(|inner| {
            let limit = inner.limit?;
            Some(limit - inner.used)
        })
    }

    /// Consume the given amount of gas. Error if the limit is exceeded.
    ///
    /// Panics if lock is poisoned.
    pub fn consume(&self, consumed: u64, comment: &'static str) -> StdResult<()> {
        self.inner.write_with(|mut inner| {
            let used = inner.used + consumed;

            // If there is a limit, and the limit is exceeded, then throw error.
            if let Some(limit) = inner.limit {
                if used > limit {
                    #[cfg(feature = "tracing")]
                    tracing::warn!(limit = inner.limit, used, comment, "Out of gas");

                    return Err(StdError::OutOfGas {
                        limit,
                        used,
                        comment,
                    });
                }
            }

            #[cfg(feature = "tracing")]
            tracing::debug!(limit = inner.limit, consumed, comment, "Gas consumed");

            inner.used = used;

            Ok(())
        })
    }
}

impl GasTracker<GLimitLess> {
    /// Return the amount of gas already used.
    ///
    /// Panics if lock is poisoned.
    pub fn used(&self) -> u64 {
        self.inner.read_access().used
    }

    /// Consume the given amount of gas.
    ///
    /// Panics if lock is poisoned.
    pub fn consumed(&self, consumed: u64) {
        self.inner.write_with(|mut inner| {
            inner.used += consumed;
        });
    }

    pub fn unbound(self) -> GasTracker<GUnbound> {
        GasTracker {
            inner: Shared::new(GUnbound {
                limit: None,
                used: self.inner.read_access().used,
            }),
        }
    }
}

impl GasTracker<GLimited> {
    pub fn limit(&self) -> u64 {
        self.inner.read_access().limit
    }

    pub fn used(&self) -> u64 {
        self.inner.read_access().used
    }

    pub fn unbound(self) -> GasTracker<GUnbound> {
        GasTracker {
            inner: Shared::new(GUnbound {
                limit: Some(self.inner.read_access().limit),
                used: self.inner.read_access().used,
            }),
        }
    }
}
impl Display for GasTracker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.read_with(|inner| {
            write!(
                f,
                "GasTracker {{ limit: {:?}, used: {} }}",
                inner.limit, inner.used
            )
        })
    }
}

pub struct GLimitLess {
    pub used: u64,
}

pub struct GLimited {
    pub limit: u64,
    pub used: u64,
}

pub struct GUnbound {
    pub limit: Option<u64>,
    pub used: u64,
}

pub trait GasTrackerMode {}
