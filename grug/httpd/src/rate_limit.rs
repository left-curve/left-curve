use {
    async_graphql::{
        BatchRequest,
        parser::types::{DocumentOperations, OperationType},
    },
    std::{
        collections::{HashMap, HashSet},
        sync::Mutex,
        time::{Duration, Instant},
    },
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GraphqlOperationCounts {
    pub queries: u32,
    pub subscriptions: u32,
}

impl GraphqlOperationCounts {
    pub fn from_batch_request(request: &mut BatchRequest) -> Self {
        let mut counts = Self {
            queries: 0,
            subscriptions: 0,
        };

        for request in request.iter_mut() {
            let operation_name = request.operation_name.clone();

            match request
                .parsed_query()
                .ok()
                .and_then(|doc| pick_operation_type(&doc.operations, operation_name.as_deref()))
            {
                Some(OperationType::Subscription) => counts.subscriptions += 1,
                Some(OperationType::Mutation) => {},
                Some(OperationType::Query) | None => counts.queries += 1,
            }
        }

        counts
    }

    pub const fn subscription() -> Self {
        Self {
            queries: 0,
            subscriptions: 1,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GraphqlIpRateLimiterConfig {
    pub window: Duration,
    pub max_queries_per_window: u32,
    pub max_subscriptions_per_window: u32,
}

impl Default for GraphqlIpRateLimiterConfig {
    fn default() -> Self {
        Self {
            window: Duration::from_secs(60),
            max_queries_per_window: 300,
            max_subscriptions_per_window: 30,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum GraphqlIpRateLimitRejection {
    Banned,
    TooManyQueries { limit: u32, attempted: u32 },
    TooManySubscriptions { limit: u32, attempted: u32 },
}

#[derive(Debug)]
pub struct GraphqlIpRateLimiter {
    config: GraphqlIpRateLimiterConfig,
    state: Mutex<GraphqlIpRateLimiterState>,
}

#[derive(Debug, Default)]
struct GraphqlIpRateLimiterState {
    banned_ips: HashSet<String>,
    activity_by_ip: HashMap<String, IpActivity>,
}

#[derive(Debug)]
struct IpActivity {
    window_started_at: Instant,
    queries: u32,
    subscriptions: u32,
}

impl GraphqlIpRateLimiter {
    pub fn new(config: GraphqlIpRateLimiterConfig) -> Self {
        Self {
            config,
            state: Mutex::new(GraphqlIpRateLimiterState::default()),
        }
    }

    pub fn ban_ip(&self, ip: impl Into<String>) {
        if let Ok(mut state) = self.state.lock() {
            state.banned_ips.insert(ip.into());
        }
    }

    pub fn unban_ip(&self, ip: &str) {
        if let Ok(mut state) = self.state.lock() {
            state.banned_ips.remove(ip);
        }
    }

    pub fn banned_ips(&self) -> Vec<String> {
        self.state
            .lock()
            .map(|state| state.banned_ips.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn check(
        &self,
        ip: Option<&str>,
        counts: GraphqlOperationCounts,
    ) -> Result<(), GraphqlIpRateLimitRejection> {
        let ip = ip.unwrap_or("unknown");
        let now = Instant::now();
        let mut state =
            self.state
                .lock()
                .map_err(|_| GraphqlIpRateLimitRejection::TooManyQueries {
                    limit: 0,
                    attempted: counts.queries,
                })?;

        if state.banned_ips.contains(ip) {
            return Err(GraphqlIpRateLimitRejection::Banned);
        }

        state.activity_by_ip.retain(|_, activity| {
            now.duration_since(activity.window_started_at) <= self.config.window
        });

        let activity = state
            .activity_by_ip
            .entry(ip.to_string())
            .or_insert_with(|| IpActivity {
                window_started_at: now,
                queries: 0,
                subscriptions: 0,
            });

        if now.duration_since(activity.window_started_at) > self.config.window {
            *activity = IpActivity {
                window_started_at: now,
                queries: 0,
                subscriptions: 0,
            };
        }

        let attempted_queries = activity.queries.saturating_add(counts.queries);
        if attempted_queries > self.config.max_queries_per_window {
            return Err(GraphqlIpRateLimitRejection::TooManyQueries {
                limit: self.config.max_queries_per_window,
                attempted: attempted_queries,
            });
        }

        let attempted_subscriptions = activity.subscriptions.saturating_add(counts.subscriptions);
        if attempted_subscriptions > self.config.max_subscriptions_per_window {
            return Err(GraphqlIpRateLimitRejection::TooManySubscriptions {
                limit: self.config.max_subscriptions_per_window,
                attempted: attempted_subscriptions,
            });
        }

        activity.queries = attempted_queries;
        activity.subscriptions = attempted_subscriptions;

        Ok(())
    }
}

impl Default for GraphqlIpRateLimiter {
    fn default() -> Self {
        Self::new(GraphqlIpRateLimiterConfig::default())
    }
}

fn pick_operation_type(
    operations: &DocumentOperations,
    operation_name: Option<&str>,
) -> Option<OperationType> {
    match operations {
        DocumentOperations::Single(operation) => Some(operation.node.ty),
        DocumentOperations::Multiple(operations) if !operations.is_empty() => {
            if let Some(name) = operation_name
                && let Some(operation) = operations.get(name)
            {
                return Some(operation.node.ty);
            }

            operations
                .values()
                .next()
                .map(|operation| operation.node.ty)
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use {crate::rate_limit::*, std::time::Duration};

    #[test]
    fn rejects_manually_banned_ip() {
        let limiter = GraphqlIpRateLimiter::new(GraphqlIpRateLimiterConfig {
            window: Duration::from_secs(60),
            max_queries_per_window: 100,
            max_subscriptions_per_window: 100,
        });

        limiter.ban_ip("198.51.100.10");

        assert_eq!(
            limiter.check(Some("198.51.100.10"), GraphqlOperationCounts {
                queries: 1,
                subscriptions: 0
            }),
            Err(GraphqlIpRateLimitRejection::Banned)
        );
    }

    #[test]
    fn rejects_queries_over_limit() {
        let limiter = GraphqlIpRateLimiter::new(GraphqlIpRateLimiterConfig {
            window: Duration::from_secs(60),
            max_queries_per_window: 1,
            max_subscriptions_per_window: 100,
        });

        assert_eq!(
            limiter.check(Some("198.51.100.10"), GraphqlOperationCounts {
                queries: 1,
                subscriptions: 0
            }),
            Ok(())
        );
        assert_eq!(
            limiter.check(Some("198.51.100.10"), GraphqlOperationCounts {
                queries: 1,
                subscriptions: 0
            }),
            Err(GraphqlIpRateLimitRejection::TooManyQueries {
                limit: 1,
                attempted: 2
            })
        );
    }

    #[test]
    fn rejects_subscriptions_over_limit() {
        let limiter = GraphqlIpRateLimiter::new(GraphqlIpRateLimiterConfig {
            window: Duration::from_secs(60),
            max_queries_per_window: 100,
            max_subscriptions_per_window: 1,
        });

        assert_eq!(
            limiter.check(
                Some("198.51.100.10"),
                GraphqlOperationCounts::subscription()
            ),
            Ok(())
        );
        assert_eq!(
            limiter.check(
                Some("198.51.100.10"),
                GraphqlOperationCounts::subscription()
            ),
            Err(GraphqlIpRateLimitRejection::TooManySubscriptions {
                limit: 1,
                attempted: 2
            })
        );
    }
}
