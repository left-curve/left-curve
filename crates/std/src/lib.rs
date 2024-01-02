// -------------------------------- all targets --------------------------------

mod db;
mod serde;
mod storage;
mod testing;
mod types;

pub use crate::{
    db::{Batch, CacheStore, Committable, Op, Order, PrefixStore, Record, Storage},
    serde::{from_json, to_json},
    storage::{Bound, Item, Map, MapKey, Path, PathBuf, Prefix, RawBound, RawKey},
    testing::MockStorage,
    types::{
        Account, Addr, Binary, BlockInfo, Coin, ContractResult, ExecuteCtx, GenesisState, Hash,
        InfoResponse, InstantiateCtx, Message, Query, QueryCtx, Response, Tx, Uint128,
        WasmRawResponse, WasmSmartResponse,
    },
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
pub use cw_std_derive::{cw_serde, entry_point};

// dependencies used by the macros
#[doc(hidden)]
pub mod __private {
    pub use ::serde;
}
