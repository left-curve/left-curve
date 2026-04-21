use {
    futures_util::{Stream, StreamExt},
    std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

/// Server-wide subscription limiter. One instance per server process.
///
/// Holds the global counter and configured limits. Each new WebSocket
/// connection gets a [`ConnectionLimiter`] that shares the global counter
/// but tracks its own per-connection count.
#[derive(Clone)]
pub struct SubscriptionLimiter {
    global_count: Arc<AtomicUsize>,
    max_per_connection: usize,
    max_global: usize,
}

impl SubscriptionLimiter {
    pub fn new(max_per_connection: usize, max_global: usize) -> Self {
        Self {
            global_count: Arc::new(AtomicUsize::new(0)),
            max_per_connection,
            max_global,
        }
    }

    /// Create a per-connection limiter that shares the global counter.
    pub fn new_connection(&self) -> ConnectionLimiter {
        ConnectionLimiter {
            connection_count: Arc::new(AtomicUsize::new(0)),
            max_per_connection: self.max_per_connection,
            global_count: self.global_count.clone(),
            max_global: self.max_global,
        }
    }
}

/// Per-connection subscription limiter injected via `GraphQLSubscription::with_data`.
pub struct ConnectionLimiter {
    connection_count: Arc<AtomicUsize>,
    max_per_connection: usize,
    global_count: Arc<AtomicUsize>,
    max_global: usize,
}

impl ConnectionLimiter {
    /// Try to acquire a subscription slot. Returns a guard that releases
    /// the slot on drop, or a GraphQL error if either limit is exceeded.
    pub fn try_acquire(&self) -> Result<SubscriptionGuard, async_graphql::Error> {
        // Optimistic increment: bump global first, roll back on failure.
        let prev_global = self.global_count.fetch_add(1, Ordering::AcqRel);
        if prev_global >= self.max_global {
            self.global_count.fetch_sub(1, Ordering::Release);
            return Err(async_graphql::Error::new(
                "Global subscription limit reached",
            ));
        }

        let prev_conn = self.connection_count.fetch_add(1, Ordering::AcqRel);
        if prev_conn >= self.max_per_connection {
            self.connection_count.fetch_sub(1, Ordering::Release);
            self.global_count.fetch_sub(1, Ordering::Release);
            return Err(async_graphql::Error::new(
                "Per-connection subscription limit reached",
            ));
        }

        Ok(SubscriptionGuard {
            connection_count: self.connection_count.clone(),
            global_count: self.global_count.clone(),
        })
    }
}

/// RAII guard that decrements both counters when the subscription stream is dropped.
pub struct SubscriptionGuard {
    connection_count: Arc<AtomicUsize>,
    global_count: Arc<AtomicUsize>,
}

impl Drop for SubscriptionGuard {
    fn drop(&mut self) {
        self.connection_count.fetch_sub(1, Ordering::Release);
        self.global_count.fetch_sub(1, Ordering::Release);
    }
}

/// Convenience helper for subscription resolvers.
///
/// Returns `Ok(guard)` that must be kept alive for the stream's lifetime.
/// If no limiter is configured (e.g. direct schema execution in tests), returns
/// `Ok` with a no-op guard. HTTP/WebSocket servers should always configure one.
pub fn acquire_subscription(
    ctx: &async_graphql::Context<'_>,
) -> Result<Option<Arc<SubscriptionGuard>>, async_graphql::Error> {
    match ctx.data_opt::<ConnectionLimiter>() {
        Some(limiter) => Ok(Some(Arc::new(limiter.try_acquire()?))),
        None => Ok(None),
    }
}

/// Attach the subscription guard to the returned stream so the slot remains
/// occupied until the subscription stream is dropped.
pub fn guard_subscription_stream<S>(
    stream: S,
    guard: Option<Arc<SubscriptionGuard>>,
) -> impl Stream<Item = S::Item>
where
    S: Stream,
{
    stream.inspect(move |_| {
        let _guard = guard.as_ref();
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforces_per_connection_limit() {
        let limiter = SubscriptionLimiter::new(1, 10);
        let connection = limiter.new_connection();

        let _guard = connection.try_acquire().unwrap();

        assert!(connection.try_acquire().is_err());
    }

    #[test]
    fn enforces_global_limit() {
        let limiter = SubscriptionLimiter::new(10, 1);
        let connection = limiter.new_connection();
        let other_connection = limiter.new_connection();

        let _guard = connection.try_acquire().unwrap();

        assert!(other_connection.try_acquire().is_err());
    }

    #[test]
    fn guarded_stream_holds_slot_until_dropped() {
        let limiter = SubscriptionLimiter::new(1, 1);
        let connection = limiter.new_connection();
        let guard = Some(Arc::new(connection.try_acquire().unwrap()));

        let stream = guard_subscription_stream(futures_util::stream::pending::<()>(), guard);

        assert!(connection.try_acquire().is_err());

        drop(stream);

        assert!(connection.try_acquire().is_ok());
    }
}
