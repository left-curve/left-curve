use {
    crate::{block_to_index::BlockToIndex, context::Context},
    async_trait::async_trait,
    grug_app::{AppError, QuerierProviderImpl, Vm},
    std::convert::Infallible,
};

#[async_trait]
pub trait Hooks {
    type Error: ToString + std::fmt::Debug;

    async fn start(&self, _context: Context) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn post_indexing<VM>(
        &self,
        _context: Context,
        _block: BlockToIndex,
        _querier: QuerierProviderImpl<VM>,
    ) -> Result<(), Self::Error>
    where
        VM: Vm + Clone + Send + Sync + 'static,
        AppError: From<VM::Error>,
    {
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
