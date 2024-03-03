use {
    crate::{
        from_json, to_json, AccountResponse, Addr, Binary, Coins, Event, GenericResult, Hash,
        InfoResponse, QueryRequest, QueryResponse, StdResult, Storage, Timestamp, Uint128, Uint64,
    },
    serde::{de::DeserializeOwned, Deserialize, Serialize},
    serde_with::skip_serializing_none,
};

pub trait Api {
    /// Verify an Secp256k1 signature with the given hashed message and public
    /// key.
    ///
    /// Note: this function takes the hash of the message, not the prehash.
    fn secp256k1_verify(
        &self,
        msg_hash: impl AsRef<[u8]>,
        sig:      impl AsRef<[u8]>,
        pk:       impl AsRef<[u8]>,
    ) -> StdResult<()>;

    /// Verify an Secp256r1 signature with the given hashed message and public
    /// key.
    ///
    /// Note: this function takes the hash of the message, not the prehash.
    fn secp256r1_verify(
        &self,
        msg_hash: impl AsRef<[u8]>,
        sig:      impl AsRef<[u8]>,
        pk:       impl AsRef<[u8]>,
    ) -> StdResult<()>;
}

pub trait Querier {
    /// Make a query. This is the only method that the context needs to manually
    /// implement. The other methods will be implemented automatically.
    fn query(&self, req: &QueryRequest) -> StdResult<QueryResponse>;

    fn query_info(&self) -> StdResult<InfoResponse> {
        self.query(&QueryRequest::Info {}).map(|res| res.as_info())
    }

    fn query_balance(&self, address: Addr, denom: String) -> StdResult<Uint128> {
        self.query(&QueryRequest::Balance {
            address,
            denom,
        })
        .map(|res| res.as_balance().amount)
    }

    fn query_balances(
        &self,
        address: Addr,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> StdResult<Coins> {
        self.query(&QueryRequest::Balances {
            address,
            start_after,
            limit,
        })
        .map(|res| res.as_balances())
    }

    fn query_supply(&self, denom: String) -> StdResult<Uint128> {
        self.query(&QueryRequest::Supply {
            denom,
        })
        .map(|res| res.as_supply().amount)
    }

    fn query_supplies(&self, start_after: Option<String>, limit: Option<u32>) -> StdResult<Coins> {
        self.query(&QueryRequest::Supplies {
            start_after,
            limit,
        })
        .map(|res| res.as_supplies())
    }

    fn query_code(&self, hash: Hash) -> StdResult<Binary> {
        self.query(&QueryRequest::Code {
            hash,
        })
        .map(|res| res.as_code())
    }

    fn query_codes(&self, start_after: Option<Hash>, limit: Option<u32>) -> StdResult<Vec<Hash>> {
        self.query(&QueryRequest::Codes {
            start_after,
            limit,
        })
        .map(|res| res.as_codes())
    }

    fn query_account(&self, address: Addr) -> StdResult<AccountResponse> {
        self.query(&QueryRequest::Account {
            address,
        })
        .map(|res| res.as_account())
    }

    fn query_accounts(
        &self,
        start_after: Option<Addr>,
        limit: Option<u32>,
    ) -> StdResult<Vec<AccountResponse>> {
        self.query(&QueryRequest::Accounts {
            start_after,
            limit,
        })
        .map(|res| res.as_accounts())
    }

    fn query_wasm_raw(&self, contract: Addr, key: Binary) -> StdResult<Option<Binary>> {
        self.query(&QueryRequest::WasmRaw {
            contract,
            key,
        })
        .map(|res| res.as_wasm_raw().value)
    }

    fn query_wasm_smart<M: Serialize, R: DeserializeOwned>(
        &self,
        contract: Addr,
        msg: &M,
    ) -> StdResult<R> {
        self.query(&QueryRequest::WasmSmart {
            contract,
            msg: to_json(msg)?,
        })
        .and_then(|res| from_json(res.as_wasm_smart().data))
    }
}

/// The context passed by the host to the Wasm module whenever an entry point is
/// called. The module then converts this to Instantiate/Execute/Query or other
/// contexts for easy usage by the contract programmer.
///
/// Some fields may be optional depending on which entry point is called.
/// For example, for queries there is no sender, because queries are not part of
/// a transaction.
#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Context {
    pub chain_id:        String,
    pub block_height:    Uint64,
    pub block_timestamp: Timestamp,
    pub block_hash:      Hash,
    pub contract:        Addr,
    pub sender:          Option<Addr>,
    pub funds:           Option<Coins>,
    pub simulate:        Option<bool>,
    pub submsg_result:   Option<GenericResult<Vec<Event>>>,
}

pub struct InstantiateCtx<'a> {
    pub store:           &'a mut dyn Storage,
    pub chain_id:        String,
    pub block_height:    Uint64,
    pub block_timestamp: Timestamp,
    pub block_hash:      Hash,
    pub contract:        Addr,
    pub sender:          Addr,
    pub funds:           Coins,
}

pub struct ExecuteCtx<'a> {
    pub store:           &'a mut dyn Storage,
    pub chain_id:        String,
    pub block_height:    Uint64,
    pub block_timestamp: Timestamp,
    pub block_hash:      Hash,
    pub contract:        Addr,
    pub sender:          Addr,
    pub funds:           Coins,
}

pub struct QueryCtx<'a> {
    pub store:           &'a dyn Storage,
    pub chain_id:        String,
    pub block_height:    Uint64,
    pub block_timestamp: Timestamp,
    pub block_hash:      Hash,
    pub contract:        Addr,
}

pub struct MigrateCtx<'a> {
    pub store:           &'a mut dyn Storage,
    pub chain_id:        String,
    pub block_height:    Uint64,
    pub block_timestamp: Timestamp,
    pub block_hash:      Hash,
    pub contract:        Addr,
    pub sender:          Addr,
}

pub struct ReplyCtx<'a> {
    pub store:           &'a mut dyn Storage,
    pub chain_id:        String,
    pub block_height:    Uint64,
    pub block_timestamp: Timestamp,
    pub block_hash:      Hash,
    pub contract:        Addr,
    pub submsg_result:   GenericResult<Vec<Event>>,
}

pub struct ReceiveCtx<'a> {
    pub store:           &'a mut dyn Storage,
    pub chain_id:        String,
    pub block_height:    Uint64,
    pub block_timestamp: Timestamp,
    pub block_hash:      Hash,
    pub contract:        Addr,
    pub sender:          Addr,
    pub funds:           Coins,
}

pub struct BeforeBlockCtx<'a> {
    pub store:           &'a mut dyn Storage,
    pub chain_id:        String,
    pub block_height:    Uint64,
    pub block_timestamp: Timestamp,
    pub block_hash:      Hash,
    pub contract:        Addr,
}

pub struct AfterBlockCtx<'a> {
    pub store:           &'a mut dyn Storage,
    pub chain_id:        String,
    pub block_height:    Uint64,
    pub block_timestamp: Timestamp,
    pub block_hash:      Hash,
    pub contract:        Addr,
}

pub struct BeforeTxCtx<'a> {
    pub store:           &'a mut dyn Storage,
    pub chain_id:        String,
    pub block_height:    Uint64,
    pub block_timestamp: Timestamp,
    pub block_hash:      Hash,
    pub contract:        Addr,
    pub simulate:        bool,
}

pub struct AfterTxCtx<'a> {
    pub store:           &'a mut dyn Storage,
    pub chain_id:        String,
    pub block_height:    Uint64,
    pub block_timestamp: Timestamp,
    pub block_hash:      Hash,
    pub contract:        Addr,
    pub simulate:        bool,
}

pub struct TransferCtx<'a> {
    pub store:           &'a mut dyn Storage,
    pub chain_id:        String,
    pub block_height:    Uint64,
    pub block_timestamp: Timestamp,
    pub block_hash:      Hash,
    pub contract:        Addr,
}

pub struct IbcClientCreateCtx<'a> {
    pub store:           &'a mut dyn Storage,
    pub chain_id:        String,
    pub block_height:    Uint64,
    pub block_timestamp: Timestamp,
    pub block_hash:      Hash,
    pub contract:        Addr,
}

pub struct IbcClientUpdateCtx<'a> {
    pub store:           &'a mut dyn Storage,
    pub chain_id:        String,
    pub block_height:    Uint64,
    pub block_timestamp: Timestamp,
    pub block_hash:      Hash,
    pub contract:        Addr,
}

pub struct IbcClientVerifyCtx<'a> {
    pub store:           &'a dyn Storage,
    pub chain_id:        String,
    pub block_height:    Uint64,
    pub block_timestamp: Timestamp,
    pub block_hash:      Hash,
    pub contract:        Addr,
}
