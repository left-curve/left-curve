use std::convert::Infallible;

use crate::context::Context;

pub trait Hooks {
    type Error: ToString + std::fmt::Debug;

    fn post_indexing(&self, _context: Context, _block_height: u64) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct NullHooks;

impl Hooks for NullHooks {
    type Error = Infallible;

    fn post_indexing(&self, _context: Context, _block_height: u64) -> Result<(), Self::Error> {
        Ok(())
    }
}
