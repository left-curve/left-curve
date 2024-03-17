// -------------------------------- all targets --------------------------------

mod context;
mod error;
mod forward_ref;
mod serde;
mod storage;
mod testing;
mod traits;
mod types;

pub use crate::{
    context::{AuthCtx, Context, ImmutableCtx, MutableCtx, SudoCtx},
    error::{StdError, StdResult},
    serde::{
        from_borsh_slice, from_json_slice, from_json_value, to_borsh_vec, to_json_value,
        to_json_vec,
    },
    storage::{
        concat, encode_length, extend_one_byte, increment_last_byte, nested_namespaces_with_key,
        split_one_key, trim, Bound, Item, Map, MapKey, Path, PathBuf, Prefix, RawBound, RawKey,
        Set,
    },
    testing::MockStorage,
    traits::{Api, Querier, Storage},
    types::{
        hash, Account, AccountResponse, Addr, Attribute, BankQueryMsg, BankQueryResponse, Batch,
        Binary, BlockInfo, ClientResponse, Coin, CoinRef, Coins, CoinsIntoIter, CoinsIter, Config,
        Decimal, Decimal256, Empty, Event, GenericResult, GenesisState, Hash, IbcClientStatus,
        IbcClientUpdateMsg, IbcClientVerifyMsg, InfoResponse, Message, Op, Order, Permission,
        Permissions, QueryRequest, QueryResponse, Record, ReplyOn, Response, SubMessage,
        SubMsgResult, Timestamp, TransferMsg, Tx, Uint128, Uint256, Uint512, Uint64,
        WasmRawResponse, WasmSmartResponse, GENESIS_BLOCK_HASH, GENESIS_SENDER,
    },
};

// ---------------------------- wasm32 target only -----------------------------

// #[cfg(target_arch = "wasm32")]
mod wasm;

// #[cfg(target_arch = "wasm32")]
pub use crate::wasm::{
    do_after_block, do_after_tx, do_bank_query, do_bank_transfer, do_before_block, do_before_tx,
    do_execute, do_ibc_client_create, do_ibc_client_update, do_ibc_client_verify, do_instantiate,
    do_migrate, do_query, do_receive, do_reply, ExternalIterator, ExternalStorage, Region,
};

// -------------------------------- re-exports ---------------------------------

/// Represents any valid JSON value, including numbers, booleans, strings,
/// objects, and arrays.
///
/// This is a re-export of `serde_json::Value`, but we name it to "Json" to be
/// clearer what it is.
pub use serde_json::Value as Json;

// proc macros
pub use cw_std_derive::{cw_derive, entry_point};

// dependencies used by the macros
#[doc(hidden)]
pub mod __private {
    pub use ::borsh;
    pub use ::serde;
    pub use ::serde_with;
}
