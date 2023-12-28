// -------------------------------- all targets --------------------------------

mod deps;
mod memory;
mod result;
mod storage;

pub use {
    crate::{
        deps::ExecuteCtx,
        memory::Region,
        result::{ContractResult, Response},
        storage::{MockStorage, Storage},
    },
    cw_std_derive::{cw_serde, entry_point},
};

// ---------------------------- wasm32 target only -----------------------------

#[cfg(target_arch = "wasm32")]
mod exports;
#[cfg(target_arch = "wasm32")]
mod imports;

#[cfg(target_arch = "wasm32")]
pub use crate::{exports::do_execute, imports::ExternalStorage};

// -------------------------------- re-exports ---------------------------------

pub mod __private {
    pub use serde;
}
