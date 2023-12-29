// -------------------------------- all targets --------------------------------

mod serde;
mod storage;
mod testing;
mod traits;
mod types;

pub use crate::{
    serde::{from_json, to_json},
    storage::{Item, Map, MapKey, Prefix, RawKey},
    testing::MockStorage,
    traits::{Order, Storage},
    types::{ContractResult, ExecuteCtx, Response},
};

// ---------------------------- wasm32 target only -----------------------------

#[cfg(target_arch = "wasm32")]
mod wasm;

#[cfg(target_arch = "wasm32")]
pub use crate::wasm::{do_execute, ExternalStorage, Region};

// -------------------------------- re-exports ---------------------------------

// macros
pub use cw_sdk_derive::{cw_serde, entry_point};

// dependencies used by the macros
#[doc(hidden)]
pub mod __private {
    pub use ::serde;
}
