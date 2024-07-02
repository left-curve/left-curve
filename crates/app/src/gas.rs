use {
    crate::{AppError, AppResult, Shared},
    std::{fmt, fmt::Display},
    tracing::{debug, warn},
};

struct GasTrackerInner {
    // `None` means there is no gas limit. This is the case during genesis, and
    // for begin/end blockers.
    limit: Option<u64>,
    used: u64,
}

#[derive(Clone)]
pub struct SharedGasTracker {
    inner: Shared<GasTrackerInner>,
}

impl SharedGasTracker {
    /// Create a new gas tracker without a gas limit.
    pub fn new_limitless() -> Self {
        Self {
            inner: Shared::new(GasTrackerInner {
                limit: None,
                used: 0,
            }),
        }
    }

    /// Create a new gas tracker with the given gas limit.
    pub fn new_limited(limit: u64) -> Self {
        Self {
            inner: Shared::new(GasTrackerInner {
                limit: Some(limit),
                used: 0,
            }),
        }
    }

    /// Return the amount of gas already used.
    ///
    /// Panics if lock is poisoned.
    pub fn used(&self) -> u64 {
        self.inner.read_access().used
    }

    /// Consume the given amount of gas. Error if the limit is exceeded.
    ///
    /// Panics if lock is poisoned.
    pub fn consume(&self, consumed: u64) -> AppResult<()> {
        self.inner.write_with(|mut inner| {
            let used = inner.used + consumed;

            // If there is a limit, and the limit is exceeded, then throw error.
            if let Some(limit) = inner.limit {
                if used > limit {
                    #[cfg(feature = "tracing")]
                    warn!(limit = inner.limit, used, "Out of gas");

                    return Err(AppError::OutOfGas { limit, used });
                }
            }

            #[cfg(feature = "tracing")]
            debug!(limit = inner.limit, used, "Gas consumed");

            inner.used = used;

            Ok(())
        })
    }
}

impl Display for SharedGasTracker {
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
