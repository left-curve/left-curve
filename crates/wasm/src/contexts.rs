use {
    grug_types::{
        from_json_value, to_json_value, AccountResponse, Addr, Api, Binary, Coins, Hash,
        InfoResponse, Querier, QueryRequest, StdResult, Storage, Timestamp, Uint128, Uint64,
    },
    serde::{de::DeserializeOwned, ser::Serialize},
};

// ----------------------------------- types -----------------------------------

/// A context that contians an immutable store. The contract is allowed to read
/// data from the store, but not write to it. This is used in query calls.
pub struct ImmutableCtx<'a> {
    pub storage: &'a dyn Storage,
    // Unlike `store`, we hide `api` and `querier`, and let user access their
    // functionalities using the methods implemented on `ctx`, for example:
    // `ctx.secp256k1_verify` instead of `ctx.api.secp256k1_verify`,
    // `ctx.query_wasm_smart` instead of `ctx.querier.query_chain`.
    // In our opinion, this is more ergonomic for users.
    #[doc(hidden)]
    pub api: &'a dyn Api,
    #[doc(hidden)]
    pub querier: &'a dyn Querier,
    pub chain_id: String,
    pub block_height: Uint64,
    pub block_timestamp: Timestamp,
    pub block_hash: Hash,
    pub contract: Addr,
}

/// A context that contains a mutable store. This is used for entry points where
/// the contract is allowed to mutate the state, such as instantiate and execute.
pub struct MutableCtx<'a> {
    pub storage: &'a mut dyn Storage,
    #[doc(hidden)]
    pub api: &'a dyn Api,
    #[doc(hidden)]
    pub querier: &'a dyn Querier,
    pub chain_id: String,
    pub block_height: Uint64,
    pub block_timestamp: Timestamp,
    pub block_hash: Hash,
    pub contract: Addr,
    pub sender: Addr,
    pub funds: Coins,
}

/// Sudo context is a state-mutable context. This is used when a contract is
/// called by the chain, instead of by a message sent by another account.
/// Therefore, compared to `MutableCtx`, it lacks the `sender` and `funds` fields.
///
/// The name is derived from the "sudo" entry point in the vanilla CosmWasm.
/// There isn't such an entry point in Grug, but we keep the name nonetheless.
pub struct SudoCtx<'a> {
    pub storage: &'a mut dyn Storage,
    #[doc(hidden)]
    pub api: &'a dyn Api,
    #[doc(hidden)]
    pub querier: &'a dyn Querier,
    pub chain_id: String,
    pub block_height: Uint64,
    pub block_timestamp: Timestamp,
    pub block_hash: Hash,
    pub contract: Addr,
}

/// Similar to `SudoCtx`, but with an additional parameter `simulate` which
/// designates whether the contract call is done in the simulation mode (e.g.
/// during the `CheckTx` ABCI call).
///
/// This is used in the `before_tx` and `after_tx` entry points, whose primary
/// purpose is to authenticate transactions, hence the name.
///
/// The typical use of the `simulate` parameter is to skip certain authentication
/// steps (e.g. verifying a cryptographic signature) if it's in simulation mode.
pub struct AuthCtx<'a> {
    pub storage: &'a mut dyn Storage,
    #[doc(hidden)]
    pub api: &'a dyn Api,
    #[doc(hidden)]
    pub querier: &'a dyn Querier,
    pub chain_id: String,
    pub block_height: Uint64,
    pub block_timestamp: Timestamp,
    pub block_hash: Hash,
    pub contract: Addr,
    pub simulate: bool,
}

// ---------------------------------- methods ----------------------------------

macro_rules! impl_methods {
    ($t:ty) => {
        impl<'a> $t {
            #[inline]
            pub fn debug(&self, msg: impl AsRef<str>) {
                self.api.debug(&self.contract, msg.as_ref())
            }

            #[inline]
            pub fn secp256k1_verify(
                &self,
                msg_hash: &[u8],
                sig: &[u8],
                pk: &[u8],
            ) -> StdResult<()> {
                self.api.secp256k1_verify(msg_hash, sig, pk)
            }

            #[inline]
            pub fn secp256r1_verify(
                &self,
                msg_hash: &[u8],
                sig: &[u8],
                pk: &[u8],
            ) -> StdResult<()> {
                self.api.secp256r1_verify(msg_hash, sig, pk)
            }

            #[inline]
            pub fn query_info(&self) -> StdResult<InfoResponse> {
                self.querier
                    .query_chain(QueryRequest::Info {})
                    .map(|res| res.as_info())
            }

            #[inline]
            pub fn query_balance(&self, address: Addr, denom: String) -> StdResult<Uint128> {
                self.querier
                    .query_chain(QueryRequest::Balance { address, denom })
                    .map(|res| res.as_balance().amount)
            }

            #[inline]
            pub fn query_balances(
                &self,
                address: Addr,
                start_after: Option<String>,
                limit: Option<u32>,
            ) -> StdResult<Coins> {
                self.querier
                    .query_chain(QueryRequest::Balances {
                        address,
                        start_after,
                        limit,
                    })
                    .map(|res| res.as_balances())
            }

            #[inline]
            pub fn query_supply(&self, denom: String) -> StdResult<Uint128> {
                self.querier
                    .query_chain(QueryRequest::Supply { denom })
                    .map(|res| res.as_supply().amount)
            }

            #[inline]
            pub fn query_supplies(
                &self,
                start_after: Option<String>,
                limit: Option<u32>,
            ) -> StdResult<Coins> {
                self.querier
                    .query_chain(QueryRequest::Supplies { start_after, limit })
                    .map(|res| res.as_supplies())
            }

            #[inline]
            pub fn query_code(&self, hash: Hash) -> StdResult<Binary> {
                self.querier
                    .query_chain(QueryRequest::Code { hash })
                    .map(|res| res.as_code())
            }

            #[inline]
            pub fn query_codes(
                &self,
                start_after: Option<Hash>,
                limit: Option<u32>,
            ) -> StdResult<Vec<Hash>> {
                self.querier
                    .query_chain(QueryRequest::Codes { start_after, limit })
                    .map(|res| res.as_codes())
            }

            #[inline]
            pub fn query_account(&self, address: Addr) -> StdResult<AccountResponse> {
                self.querier
                    .query_chain(QueryRequest::Account { address })
                    .map(|res| res.as_account())
            }

            #[inline]
            pub fn query_accounts(
                &self,
                start_after: Option<Addr>,
                limit: Option<u32>,
            ) -> StdResult<Vec<AccountResponse>> {
                self.querier
                    .query_chain(QueryRequest::Accounts { start_after, limit })
                    .map(|res| res.as_accounts())
            }

            #[inline]
            pub fn query_wasm_raw(&self, contract: Addr, key: Binary) -> StdResult<Option<Binary>> {
                self.querier
                    .query_chain(QueryRequest::WasmRaw { contract, key })
                    .map(|res| res.as_wasm_raw().value)
            }

            #[inline]
            pub fn query_wasm_smart<M: Serialize, R: DeserializeOwned>(
                &self,
                contract: Addr,
                msg: &M,
            ) -> StdResult<R> {
                self.querier
                    .query_chain(QueryRequest::WasmSmart {
                        contract,
                        msg: to_json_value(msg)?,
                    })
                    .and_then(|res| from_json_value(res.as_wasm_smart().data))
            }
        }
    };
}

impl_methods!(ImmutableCtx<'a>);
impl_methods!(MutableCtx<'a>);
impl_methods!(SudoCtx<'a>);
impl_methods!(AuthCtx<'a>);
