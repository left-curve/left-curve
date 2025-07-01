use {http::Extensions, std::sync::Arc};

/// Context that gets passed between indexer hooks to share data
#[derive(Debug, Clone)]
pub struct IndexerContext {
    /// Shared data storage using http::Extensions - much simpler than TypedDashMap
    data: Arc<std::sync::Mutex<Extensions>>,
    /// Metadata about the current indexing operation
    metadata: IndexerMetadata,
}

/// Metadata about the current indexing operation
#[derive(Debug, Clone, Default)]
pub struct IndexerMetadata {
    /// The current block height being processed (if any)
    pub current_block_height: Option<u64>,
    /// Arbitrary key-value metadata
    pub properties: std::collections::HashMap<String, String>,
}

impl IndexerContext {
    /// Create a new empty context
    pub fn new() -> Self {
        Self {
            data: Arc::new(std::sync::Mutex::new(Extensions::new())),
            metadata: IndexerMetadata::default(),
        }
    }

    /// Get access to the shared data storage
    pub fn data(&self) -> &Arc<std::sync::Mutex<Extensions>> {
        &self.data
    }

    /// Mutable access to the shared data storage
    pub fn data_mut(&mut self) -> &mut Arc<std::sync::Mutex<Extensions>> {
        &mut self.data
    }

    /// Get a reference to the metadata
    pub fn metadata(&self) -> &IndexerMetadata {
        &self.metadata
    }

    /// Get a mutable reference to the metadata
    pub fn metadata_mut(&mut self) -> &mut IndexerMetadata {
        &mut self.metadata
    }

    /// Set the current block height
    pub fn set_current_block_height(&mut self, height: u64) {
        self.metadata.current_block_height = Some(height);
    }

    /// Get the current block height
    pub fn current_block_height(&self) -> Option<u64> {
        self.metadata.current_block_height
    }

    /// Set a metadata property
    pub fn set_property(&mut self, key: String, value: String) {
        self.metadata.properties.insert(key, value);
    }

    /// Get a metadata property
    pub fn get_property(&self, key: &str) -> Option<&String> {
        self.metadata.properties.get(key)
    }

    /// Remove a metadata property
    pub fn remove_property(&mut self, key: &str) -> Option<String> {
        self.metadata.properties.remove(key)
    }
}

impl Default for IndexerContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() {
        let context = IndexerContext::new();
        assert_eq!(context.current_block_height(), None);
    }

    #[test]
    fn test_data_operations() {
        let context = IndexerContext::new();

        // Test setting and getting data with Extensions
        context.data().lock().unwrap().insert(42i32);
        context.data().lock().unwrap().insert("hello".to_string());

        assert_eq!(context.data().lock().unwrap().get::<i32>(), Some(&42));
        assert_eq!(
            context.data().lock().unwrap().get::<String>(),
            Some(&"hello".to_string())
        );
        assert_eq!(context.data().lock().unwrap().get::<u64>(), None);
    }

    #[test]
    fn test_metadata_operations() {
        let mut context = IndexerContext::new();

        // Test block height
        assert_eq!(context.current_block_height(), None);
        context.set_current_block_height(100);
        assert_eq!(context.current_block_height(), Some(100));

        // Test properties
        assert_eq!(context.get_property("key1"), None);
        context.set_property("key1".to_string(), "value1".to_string());
        assert_eq!(context.get_property("key1"), Some(&"value1".to_string()));

        assert_eq!(context.remove_property("key1"), Some("value1".to_string()));
        assert_eq!(context.get_property("key1"), None);
    }

    #[test]
    fn test_context_cloning() {
        let mut context1 = IndexerContext::new();
        context1.data().lock().unwrap().insert(42i32);
        context1.set_current_block_height(100);
        context1.set_property("key".to_string(), "value".to_string());

        let context2 = context1.clone();

        assert_eq!(context2.data().lock().unwrap().get::<i32>(), Some(&42));
        assert_eq!(context2.current_block_height(), Some(100));
        assert_eq!(context2.get_property("key"), Some(&"value".to_string()));
    }
}
