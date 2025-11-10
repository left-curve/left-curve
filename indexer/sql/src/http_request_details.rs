use {
    grug_types::HttpRequestDetails,
    std::{
        collections::HashMap,
        ops::{Deref, DerefMut},
        time::{SystemTime, UNIX_EPOCH},
    },
};

#[derive(Debug)]
pub struct HttpRequestDetailsCache {
    map: HashMap<String, HttpRequestDetails>,
    max_items: usize,
}

impl Default for HttpRequestDetailsCache {
    fn default() -> Self {
        Self {
            map: HashMap::new(),
            max_items: 1000,
        }
    }
}

impl Deref for HttpRequestDetailsCache {
    type Target = HashMap<String, HttpRequestDetails>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl DerefMut for HttpRequestDetailsCache {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}

impl HttpRequestDetailsCache {
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
