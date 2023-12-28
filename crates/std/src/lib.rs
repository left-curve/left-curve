mod context;
mod db;
#[cfg(target_arch = "wasm32")]
mod exports;
mod memory;
mod result;

pub use {
    crate::{
        context::ExecuteCtx,
        db::{ExternalStorage, MockStorage, Storage},
        memory::Region,
        result::{ContractResult, Response},
    },
    cw_std_derive::{cw_serde, entry_point},
};

#[cfg(target_arch = "wasm32")]
pub use crate::exports::do_execute;

// re-export, for use in macro expansions
pub mod __private {
    pub use serde;
}
