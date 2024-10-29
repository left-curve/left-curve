use {
    crate::Shared,
    grug_types::{Defined, MaybeDefined, StdError, StdResult, Undefined},
    std::fmt::{self, Display},
};

/// A numerical measurement of the amount of computing resourced consumed.
pub type Gas = u64;

/// Tracks gas consumption; throws error if gas limit is exceeded.
#[derive(Clone)]
pub struct GasTracker<L> {
    inner: Shared<GasTrackerInner<L>>,
}

struct GasTrackerInner<L> {
    // An undefined gas limit means there is no limit.
    // This is the case for genesis, cronjobs, and `finalize_fee` calls.
    limit: L,
    used: Gas,
}

impl GasTracker<Undefined<Gas>> {
    /// Create a new gas tracker without a gas limit.
    pub fn new_limitless() -> Self {
        Self {
            inner: Shared::new(GasTrackerInner {
                limit: Undefined::new(),
                used: 0,
            }),
        }
    }
}

impl GasTracker<Defined<Gas>> {
    /// Create a new gas tracker with the given gas limit.
    pub fn new_limited(limit: Gas) -> Self {
        Self {
            inner: Shared::new(GasTrackerInner {
                limit: Defined::new(limit),
                used: 0,
            }),
        }
    }

    /// Return the gas limit.
    ///
    /// Panics if lock is poisoned.
    pub fn limit(&self) -> Gas {
        self.inner.read_access().limit.into_inner()
    }

    /// Return the amount of gas remaining.
    ///
    /// Panics if lock is poisoned.
    pub fn remaining(&self) -> Gas {
        self.inner
            .read_with(|inner| inner.limit.into_inner() - inner.used)
    }
}

impl<L> GasTracker<L>
where
    L: MaybeDefined<Gas>,
{
    /// Return the amount of gas already used.
    ///
    /// Panics if lock is poisoned.
    pub fn used(&self) -> Gas {
        self.inner.read_access().used
    }

    /// Return the gas limit. `None` if there isn't a limit.
    ///
    /// Panics if lock is poisoned.
    pub fn maybe_limit(&self) -> Option<Gas> {
        self.inner.read_access().limit.maybe_inner().copied()
    }

    /// Return the amount of gas remaining. `None` if there isn't a limit.
    ///
    /// Panics if lock is poisoned.
    pub fn maybe_remaining(&self) -> Option<Gas> {
        self.inner
            .read_with(|inner| inner.limit.maybe_inner().map(|limit| *limit - inner.used))
    }

    /// Consume the given amount of gas. Error if the limit is exceeded.
    ///
    /// Panics if lock is poisoned.
    pub fn consume(&self, consumed: Gas, comment: &'static str) -> StdResult<()> {
        self.inner.write_with(|mut inner| {
            let used = inner.used + consumed;

            // If there is a limit, and the limit is exceeded, then throw error.
            if let Some(limit) = inner.limit.maybe_inner().copied() {
                if used > limit {
                    #[cfg(feature = "tracing")]
                    tracing::warn!(limit, used, comment, "Out of gas");

                    return Err(StdError::OutOfGas {
                        limit,
                        used,
                        comment,
                    });
                }
            }

            #[cfg(feature = "tracing")]
            tracing::debug!(
                limit = inner.limit.maybe_inner(),
                consumed,
                comment,
                "Gas consumed"
            );

            inner.used = used;

            Ok(())
        })
    }
}

impl<L> Display for GasTracker<L>
where
    L: MaybeDefined<Gas>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.read_with(|inner| {
            write!(
                f,
                "GasTracker {{ limit: {:?}, used: {} }}",
                inner.limit.maybe_inner(),
                inner.used
            )
        })
    }
}
