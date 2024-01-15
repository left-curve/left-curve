// -------------------------------- all targets --------------------------------

mod error;
mod serde;
mod storage;
mod testing;
mod types;

pub use crate::{
    error::{StdError, StdResult},
    serde::{from_json, to_json},
    storage::{Bound, Item, Map, MapKey, Path, PathBuf, Prefix, RawBound, RawKey},
    types::{
        hash, Account, AccountResponse, Addr, Attribute, BeforeTxCtx, Binary, BlockInfo, Coin,
        Config, Context, ExecuteCtx, GenericResult, GenesisState, Hash, InfoResponse,
        InstantiateCtx, Message, QueryCtx, QueryRequest, QueryResponse, Response, Tx, Uint128,
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
    do_before_tx, do_execute, do_instantiate, do_query, ExternalIterator, ExternalStorage, Region,
};

// -------------------------------- re-exports ---------------------------------

// stuff from cw-db
pub use cw_db::{Batch, Op, Order, Record, Storage};

// macros
pub use cw_std_derive::{cw_serde, entry_point};

// dependencies used by the macros
#[doc(hidden)]
pub mod __private {
    pub use ::serde;
}
