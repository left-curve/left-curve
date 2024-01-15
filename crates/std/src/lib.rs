// -------------------------------- all targets --------------------------------

mod error;
mod serde;
mod storage;
mod testing;
mod types;

pub use crate::{
    error::{StdError, StdResult},
    serde::{from_json, to_json},
    storage::{
        Bound, Item, Map, MapKey, Order, Path, PathBuf, Prefix, RawBound, RawKey, Record, Storage,
    },
    testing::MockStorage,
    types::{
        hash, Account, AccountResponse, Addr, Attribute, BeforeTxCtx, Binary, BlockInfo, Coin,
        Config, ExecuteCtx, GenericResult, GenesisState, Hash, InfoResponse, InstantiateCtx,
        Message, QueryCtx, QueryRequest, QueryResponse, Response, Tx, Uint128, WasmRawResponse,
        WasmSmartResponse,
    },
};

// TODO: put this under an optional feature
pub use crate::types::Context;

#[cfg(feature = "storage-utils")]
pub mod storage_utils {
    pub use crate::storage::{
        concat, encode_length, extend_one_byte, increment_last_byte, nested_namespaces_with_key,
        split_one_key, trim,
    };
}

// ---------------------------- wasm32 target only -----------------------------
// note: during development, it's helpful to comment out the target_arch tags,
// otherwise rust-analyzer won't include these files.

// #[cfg(target_arch = "wasm32")]
mod wasm;

// #[cfg(target_arch = "wasm32")]
pub use crate::wasm::{
    do_before_tx, do_execute, do_instantiate, do_query, ExternalIterator, ExternalStorage, Region,
};

// -------------------------------- re-exports ---------------------------------

// macros
pub use cw_std_derive::{cw_serde, entry_point};

// dependencies used by the macros
#[doc(hidden)]
pub mod __private {
    pub use ::serde;
}
