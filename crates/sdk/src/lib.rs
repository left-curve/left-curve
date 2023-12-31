// -------------------------------- all targets --------------------------------

mod serde;
mod storage;
mod testing;
mod traits;
mod types;

pub use crate::{
    serde::{from_json, to_json},
    storage::{Item, Map, MapKey, Path, PathBuf, Prefix, RawKey},
    testing::MockStorage,
    traits::{Order, Storage},
    types::{Binary, ContractResult, ExecuteCtx, InstantiateCtx, QueryCtx, Response, Uint128},
};

// ---------------------------- wasm32 target only -----------------------------
// note: during development, it's helpful to comment out the target_arch tags,
// otherwise rust-analyzer won't include these files.

// #[cfg(target_arch = "wasm32")]
mod wasm;

// #[cfg(target_arch = "wasm32")]
pub use crate::wasm::{
    do_execute, do_instantiate, do_query, ExternalIterator, ExternalStorage, Region,
};

// -------------------------------- re-exports ---------------------------------

// macros
pub use cw_sdk_derive::{cw_serde, entry_point};

// dependencies used by the macros
#[doc(hidden)]
pub mod __private {
    pub use ::serde;
}
