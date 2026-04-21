use {
    crate::{rate_limit::GraphqlIpRateLimiter, traits::QueryApp},
    std::sync::Arc,
};

#[derive(Clone)]
pub struct Context {
    pub grug_app: Arc<dyn QueryApp + Send + Sync>,
    pub graphql_rate_limiter: Arc<GraphqlIpRateLimiter>,
}

impl Context {
    pub fn new(grug_app: Arc<dyn QueryApp + Send + Sync>) -> Self {
        Self::new_with_graphql_rate_limiter(grug_app, Arc::new(GraphqlIpRateLimiter::default()))
    }

    pub fn new_with_graphql_rate_limiter(
        grug_app: Arc<dyn QueryApp + Send + Sync>,
        graphql_rate_limiter: Arc<GraphqlIpRateLimiter>,
    ) -> Self {
        Self {
            grug_app,
            graphql_rate_limiter,
        }
    }

    pub fn ban_graphql_ip(&self, ip: impl Into<String>) {
        self.graphql_rate_limiter.ban_ip(ip);
    }

    pub fn unban_graphql_ip(&self, ip: &str) {
        self.graphql_rate_limiter.unban_ip(ip);
    }

    pub fn banned_graphql_ips(&self) -> Vec<String> {
        self.graphql_rate_limiter.banned_ips()
    }
}
