use {
    grug_types::{Block, BlockOutcome, Storage},
    std::sync::Arc,
};

use crate::{IndexerError, QuerierProvider};

/// Result type for indexer operations
pub type IndexerResult<T> = Result<T, IndexerError>;

/// Context for passing data between indexers in a composite pattern.
/// Uses http::Extensions to allow storing arbitrary typed data that can be
/// shared between different indexer implementations.
///
/// # Example
/// ```ignore
/// // In an earlier indexer implementation:
/// ctx.insert("shared_data".to_string());
///
/// // In a later indexer implementation:
/// if let Some(data) = ctx.get::<String>() {
///     println!("Received: {}", data);
/// }
/// ```
///
/// Note: Types must implement `Clone + Send + Sync + 'static` to be stored.
#[derive(Debug, Default, Clone)]
pub struct IndexerContext {
    extensions: http::Extensions,
}

impl IndexerContext {
    /// Create a new empty context
    pub fn new() -> Self {
        Self {
            extensions: http::Extensions::new(),
        }
    }

    /// Insert a value into the context
    pub fn insert<T: Clone + Send + Sync + 'static>(&mut self, value: T) -> Option<T> {
        self.extensions.insert(value)
    }

    /// Get a reference to a value from the context
    pub fn get<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.extensions.get::<T>()
    }

    /// Get a mutable reference to a value from the context
    pub fn get_mut<T: Send + Sync + 'static>(&mut self) -> Option<&mut T> {
        self.extensions.get_mut::<T>()
    }

    /// Remove a value from the context
    pub fn remove<T: Send + Sync + 'static>(&mut self) -> Option<T> {
        self.extensions.remove::<T>()
    }

    /// Check if the context contains a value of type T
    pub fn contains<T: Send + Sync + 'static>(&self) -> bool {
        self.extensions.get::<T>().is_some()
    }
}

/// This is the trait that the indexer must implement. It is used by the Grug core to index blocks
pub trait Indexer {
    /// Called when initializing the indexer, allowing for DB migration if needed
    fn start(&mut self, _storage: &dyn Storage) -> IndexerResult<()> {
        Ok(())
    }

    /// Called when terminating the indexer, allowing for DB transactions to be committed
    fn shutdown(&mut self) -> IndexerResult<()>;

    /// Called when indexing a block, allowing to create a new DB transaction
    fn pre_indexing(&self, block_height: u64, ctx: &mut IndexerContext) -> IndexerResult<()>;

    /// Called when indexing the block, happens at the end of the block creation
    fn index_block(
        &self,
        block: &Block,
        block_outcome: &BlockOutcome,
        ctx: &mut IndexerContext,
    ) -> IndexerResult<()>;

    /// Called after indexing the block, allowing for DB transactions to be committed
    /// Uses owned querier to allow spawning in background threads
    fn post_indexing(
        &self,
        block_height: u64,
        querier: Arc<dyn QuerierProvider>,
        ctx: &mut IndexerContext,
    ) -> IndexerResult<()>;

    /// Wait for the indexer to finish indexing
    fn wait_for_finish(&self) -> IndexerResult<()>;
}
