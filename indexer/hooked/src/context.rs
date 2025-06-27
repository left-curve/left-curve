use {crate::TypeMap, std::sync::Arc};

/// Context that gets passed between indexer hooks to share data
#[derive(Debug, Clone)]
pub struct IndexerContext {
    /// Shared data store for hooks to communicate - uses TypedDashMap for type-safe storage
    data: Arc<TypeMap>,
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
            data: Arc::new(TypeMap::new()),
            metadata: IndexerMetadata::default(),
        }
    }

    /// Get direct access to the typed data store
    ///
    /// Example usage:
    /// ```
    /// use indexer_hooked::IndexerContext;
    /// use typedmap::TypedMapKey;
    ///
    /// #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    /// struct UserId(u64);
    ///
    /// impl TypedMapKey for UserId {
    ///     type Value = String;
    /// }
    ///
    /// let context = IndexerContext::new();
    ///
    /// // Store data
    /// context.data().insert(UserId(123), "alice".to_string());
    ///
    /// // Retrieve data
    /// let username = context.data().get(&UserId(123));
    /// ```
    pub fn data(&self) -> &TypeMap {
        &self.data
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
    use {super::*, typedmap::TypedMapKey};

    // Test types for TypedMapKey
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    struct TestCounter(u32);

    impl TypedMapKey for TestCounter {
        type Value = i32;
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    struct TestMessage(String);

    impl TypedMapKey for TestMessage {
        type Value = String;
    }

    #[test]
    fn test_context_creation() {
        let context = IndexerContext::new();
        assert_eq!(context.current_block_height(), None);
        assert!(!context.data().contains_key(&TestCounter(1)));
    }

    #[test]
    fn test_data_operations() {
        let context = IndexerContext::new();

        // Test setting and getting data
        context.data().insert(TestCounter(1), 42);
        context
            .data()
            .insert(TestMessage("key".to_string()), "hello".to_string());

        assert_eq!(
            context.data().get(&TestCounter(1)).map(|v| *v.value()),
            Some(42)
        );
        assert_eq!(
            context
                .data()
                .get(&TestMessage("key".to_string()))
                .map(|v| v.value().clone()),
            Some("hello".to_string())
        );
        assert!(context.data().get(&TestCounter(2)).is_none());

        // Test contains_key
        assert!(context.data().contains_key(&TestCounter(1)));
        assert!(context.data().contains_key(&TestMessage("key".to_string())));
        assert!(!context.data().contains_key(&TestCounter(2)));

        // Test remove
        let removed = context.data().remove(&TestCounter(1));
        assert!(removed.is_some());
        assert!(!context.data().contains_key(&TestCounter(1)));
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
        context1.data().insert(TestCounter(1), 42);
        context1.set_current_block_height(100);
        context1.set_property("key".to_string(), "value".to_string());

        let context2 = context1.clone();

        assert_eq!(
            context2.data().get(&TestCounter(1)).map(|v| *v.value()),
            Some(42)
        );
        assert_eq!(context2.current_block_height(), Some(100));
        assert_eq!(context2.get_property("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_direct_data_access() {
        let context = IndexerContext::new();

        // Direct access is much cleaner
        context.data().insert(TestCounter(1), 42);

        let value = context.data().get(&TestCounter(1)).map(|v| *v.value());
        assert_eq!(value, Some(42));

        // Update the value
        context.data().insert(TestCounter(1), 100);
        let updated_value = context.data().get(&TestCounter(1)).map(|v| *v.value());
        assert_eq!(updated_value, Some(100));
    }
}
