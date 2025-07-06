use {
    crate::{block_to_index::BlockToIndex, context::Context},
    async_trait::async_trait,
    grug_app::QuerierProvider,
    std::convert::Infallible,
};

#[async_trait]
pub trait Hooks {
    type Error: ToString + std::fmt::Debug;

    async fn start(&self, _context: Context) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn post_indexing(
        &self,
        _context: Context,
        _block: BlockToIndex,
        _querier: &dyn QuerierProvider,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn shutdown(&self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct NullHooks;

impl Hooks for NullHooks {
    type Error = Infallible;
}
