use {
    super::http_request_details::HttpRequestDetails,
    crate::Hash256,
    std::{
        collections::HashMap,
        ops::{Deref, DerefMut},
        time::{SystemTime, UNIX_EPOCH},
    },
};

/// Stores transactions hash to HttpRequestDetails mapping, with a cleaning
/// mechanism to avoid unbounded memory growth
#[derive(Debug)]
pub struct TransactionsHttpdRequest {
    map: HashMap<Hash256, HttpRequestDetails>,
    max_items: usize,
}

impl Default for TransactionsHttpdRequest {
    fn default() -> Self {
        Self {
            map: HashMap::new(),
            max_items: 1000,
        }
    }
}

impl Deref for TransactionsHttpdRequest {
    type Target = HashMap<Hash256, HttpRequestDetails>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl DerefMut for TransactionsHttpdRequest {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}

impl TransactionsHttpdRequest {
    /// Remove old entries, and increase the max_items if needed so it doesn't
    /// keep cleaning for each request
    pub fn clean(&mut self) {
        if self.map.len() <= self.max_items {
            return;
        }

        // keep 20sec old entries is way enough for transactions to be indexed.
        let cutoff = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            - 20;

        self.map.retain(|_, value| value.created_at >= cutoff);

        self.max_items = (self.map.len() * 2).max(1000);
    }
}
