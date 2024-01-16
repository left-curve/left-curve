use {
    crate::{ACCOUNTS, CODES, CONFIG, CONTRACT_NAMESPACE, LAST_FINALIZED_BLOCK},
    cw_db::{BackendStorage, PrefixStore, SharedStore},
    cw_std::{
        AccountResponse, Addr, Binary, BlockInfo, Bound, Context, GenericResult, Hash,
        InfoResponse, Order, QueryRequest, QueryResponse, Storage, WasmRawResponse,
        WasmSmartResponse,
    },
    cw_vm::{BackendQuerier, Instance, VmResult},
};

const DEFAULT_PAGE_LIMIT: u32 = 30;

// ------------------------------ backend querier ------------------------------

pub struct Querier<S> {
    store: SharedStore<S>,
    block: BlockInfo,
}

impl<S> Querier<S> {
    pub fn new(store: SharedStore<S>, block: BlockInfo) -> Self {
        Self { store, block }
    }
}

impl<S: Storage + 'static> BackendQuerier for Querier<S> {
    fn query_chain(&self, req: QueryRequest) -> VmResult<GenericResult<QueryResponse>> {
        Ok(process_query(self.store.share(), &self.block, req).into())
    }
}

// ------------------------------- process query -------------------------------

pub fn process_query<S: Storage + 'static>(
    store: SharedStore<S>,
    block: &BlockInfo,
    req:   QueryRequest,
) -> anyhow::Result<QueryResponse> {
    match req {
        QueryRequest::Info {} => query_info(&store).map(QueryResponse::Info),
        QueryRequest::Code {
            hash,
        } => query_code(&store, hash).map(QueryResponse::Code),
        QueryRequest::Codes {
            start_after,
            limit,
        } => query_codes(&store, start_after, limit).map(QueryResponse::Codes),
        QueryRequest::Account {
            address,
        } => query_account(&store, address).map(QueryResponse::Account),
        QueryRequest::Accounts {
            start_after,
            limit,
        } => query_accounts(&store, start_after, limit).map(QueryResponse::Accounts),
        QueryRequest::WasmRaw {
            contract,
            key,
        } => query_wasm_raw(store, contract, key).map(QueryResponse::WasmRaw),
        QueryRequest::WasmSmart {
            contract,
            msg
        } => query_wasm_smart(store, block, contract, msg).map(QueryResponse::WasmSmart),
    }
}

fn query_info(store: &dyn Storage) -> anyhow::Result<InfoResponse> {
    Ok(InfoResponse {
        config:               CONFIG.load(store)?,
        last_finalized_block: LAST_FINALIZED_BLOCK.load(store)?,
    })
}

fn query_code(store: &dyn Storage, hash: Hash) -> anyhow::Result<Binary> {
    CODES.load(store, &hash)
}

fn query_codes(
    store:       &dyn Storage,
    start_after: Option<Hash>,
    limit:       Option<u32>,
) -> anyhow::Result<Vec<Hash>> {
    let start = start_after.as_ref().map(Bound::exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    CODES
        .keys(store, start, None, Order::Ascending)
        .take(limit as usize)
        .collect()
}

fn query_account(store: &dyn Storage, address: Addr) -> anyhow::Result<AccountResponse> {
    let account = ACCOUNTS.load(store, &address)?;
    Ok(AccountResponse {
        address,
        code_hash: account.code_hash,
        admin:     account.admin,
    })
}

fn query_accounts(
    store:       &dyn Storage,
    start_after: Option<Addr>,
    limit:       Option<u32>,
) -> anyhow::Result<Vec<AccountResponse>> {
    let start = start_after.as_ref().map(Bound::exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    ACCOUNTS
        .range(store, start, None, Order::Ascending)
        .take(limit as usize)
        .map(|item| {
            let (address, account) = item?;
            Ok(AccountResponse {
                address,
                code_hash: account.code_hash,
                admin:     account.admin,
            })
        })
        .collect()
}

fn query_wasm_raw<S: Storage + 'static>(
    store:    S,
    contract: Addr,
    key:      Binary,
) -> anyhow::Result<WasmRawResponse> {
    let substore = PrefixStore::new(store, &[CONTRACT_NAMESPACE, contract.as_ref()]);
    let value = substore.read(key.as_ref())?;
    Ok(WasmRawResponse {
        contract,
        key,
        value: value.map(Binary::from),
    })
}

fn query_wasm_smart<S: Storage + 'static>(
    store:    SharedStore<S>,
    block:    &BlockInfo,
    contract: Addr,
    msg:      Binary,
) -> anyhow::Result<WasmSmartResponse> {
    // load wasm code
    let account = ACCOUNTS.load(&store, &contract)?;
    let wasm_byte_code = CODES.load(&store, &account.code_hash)?;

    // create wasm host
    let substore = PrefixStore::new(store.share(), &[CONTRACT_NAMESPACE, contract.as_ref()]);
    let querier = Querier::new(store, block.clone());
    let mut instance = Instance::build_from_code(substore, querier, wasm_byte_code.as_ref())?;

    // call query
    let ctx = Context {
        block:    block.clone(),
        sender:   None,
        simulate: None,
        contract,
    };
    let data = instance.call_query(&ctx, msg)?.into_std_result()?;

    Ok(WasmSmartResponse {
        contract: ctx.contract,
        data,
    })
}
