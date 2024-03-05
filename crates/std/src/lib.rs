// -------------------------------- all targets --------------------------------

mod error;
mod forward_ref;
mod serde;
mod storage;
mod testing;
mod traits;
mod types;

pub use crate::{
    error::{StdError, StdResult},
    serde::{from_borsh, from_json, to_borsh, to_json},
    storage::{
        concat, encode_length, extend_one_byte, increment_last_byte, nested_namespaces_with_key,
        split_one_key, trim, Bound, Item, Map, MapKey, Path, PathBuf, Prefix, RawBound, RawKey,
        Set,
    },
    testing::MockStorage,
    traits::{Api, Querier, Storage},
    types::{
        hash, Account, AccountResponse, Addr, AfterBlockCtx, AfterTxCtx, Attribute, BankQuery,
        BankQueryResponse, Batch, BeforeBlockCtx, BeforeTxCtx, Binary, BlockInfo, ClientResponse,
        Coin, CoinRef, Coins, CoinsIntoIter, CoinsIter, Config, Context, Decimal, Decimal256,
        Empty, Event, ExecuteCtx, GenericResult, GenesisState, Hash, IbcClientExecuteMsg,
        IbcClientQueryMsg, IbcClientQueryResponse, IbcClientStateResponse, IbcClientStatus,
        InfoResponse, InstantiateCtx, Message, MigrateCtx, Op, Order, Permission, QueryCtx,
        QueryRequest, QueryResponse, ReceiveCtx, Record, ReplyCtx, ReplyOn, Response, SubMessage,
        Timestamp, TransferCtx, TransferMsg, Tx, Uint128, Uint256, Uint512, Uint64,
        WasmRawResponse, WasmSmartResponse, GENESIS_BLOCK_HASH, GENESIS_SENDER,
    },
};

// ---------------------------- wasm32 target only -----------------------------

// #[cfg(target_arch = "wasm32")]
mod wasm;

// #[cfg(target_arch = "wasm32")]
pub use crate::wasm::{
    do_after_block, do_after_tx, do_before_block, do_before_tx, do_execute, do_instantiate,
    do_migrate, do_query, do_query_bank, do_receive, do_reply, do_transfer, ExternalIterator,
    ExternalStorage, Region,
};

// -------------------------------- re-exports ---------------------------------

// macros
pub use cw_std_derive::{cw_derive, entry_point};

// dependencies used by the macros
#[doc(hidden)]
pub mod __private {
    pub use ::borsh;
    pub use ::serde;
    pub use ::serde_with;
}
