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
        hash, Account, AccountResponse, Addr, Attribute, BankQuery, BankQueryResponse, BeforeTxCtx,
        Binary, BlockInfo, Coin, CoinRef, Coins, CoinsIntoIter, CoinsIter, Config, Context, Empty,
        Event, ExecuteCtx, GenericResult, GenesisState, Hash, InfoResponse, InstantiateCtx,
        Message, MigrateCtx, QueryCtx, QueryRequest, QueryResponse, ReceiveCtx, ReplyCtx, ReplyOn,
        Response, SubMessage, TransferCtx, TransferMsg, Tx, Uint128, WasmRawResponse,
        WasmSmartResponse,
    },
};

// ---------------------------- wasm32 target only -----------------------------

// #[cfg(target_arch = "wasm32")]
mod wasm;

// #[cfg(target_arch = "wasm32")]
pub use crate::wasm::{
    do_before_tx, do_execute, do_instantiate, do_migrate, do_query, do_query_bank, do_receive,
    do_reply, do_transfer, ExternalIterator, ExternalStorage, Region,
};

// -------------------------------- re-exports ---------------------------------

// stuff from cw-db
pub use cw_db::{Batch, MockStorage, Op, Order, Record, Storage};

// macros
pub use cw_std_derive::{cw_serde, entry_point};

// dependencies used by the macros
#[doc(hidden)]
pub mod __private {
    pub use ::serde;
}
