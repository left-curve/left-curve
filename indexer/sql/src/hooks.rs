use {
    crate::{block_to_index::BlockToIndex, context::Context},
    async_trait::async_trait,
    grug_app::QuerierProvider,
    std::convert::Infallible,
};

/// The main Hooks trait with associated Error type for type safety
#[async_trait]
pub trait Hooks: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    /// Returns a name for this hook, used for logging and debugging
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    async fn start(&self, _context: Context) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn post_indexing(
        &self,
        _context: Context,
        _block: BlockToIndex,
        _querier: &dyn QuerierProvider,
    ) -> Result<(), Self::Error>;

    async fn shutdown(&self) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// Object-safe trait for dynamic dispatch
#[async_trait]
pub trait DynHooks: Send + Sync {
    /// Returns a name for this hook, used for logging and debugging
    fn name(&self) -> &str;

    async fn start(&self, context: Context)
    -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    async fn post_indexing(
        &self,
        context: Context,
        block: BlockToIndex,
        querier: &dyn QuerierProvider,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

/// Wrapper that implements DynHooks for any type that implements Hooks
pub struct HookWrapper<H> {
    inner: H,
}

impl<H> HookWrapper<H> {
    pub fn new(hook: H) -> Self {
        Self { inner: hook }
    }
}

#[async_trait]
impl<H: Hooks> DynHooks for HookWrapper<H> {
    fn name(&self) -> &str {
        self.inner.name()
    }

    async fn start(
        &self,
        context: Context,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.inner
            .start(context)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn post_indexing(
        &self,
        context: Context,
        block: BlockToIndex,
        querier: &dyn QuerierProvider,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.inner
            .post_indexing(context, block, querier)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.inner
            .shutdown()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }
}

#[derive(Debug, Clone, Default)]
pub struct NullHooks;

#[async_trait]
impl Hooks for NullHooks {
    type Error = Infallible;

    async fn post_indexing(
        &self,
        _context: Context,
        _block: BlockToIndex,
        _querier: &dyn QuerierProvider,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}
