use {
    crate::{ACCOUNTS, CODES, CONFIG, CONTRACT_NAMESPACE, LAST_FINALIZED_BLOCK},
    cw_db::{BackendStorage, PrefixStore},
    cw_std::{
        AccountResponse, Addr, BankQuery, BankQueryResponse, Binary, BlockInfo, Bound, Coin, Coins,
        Context, GenericResult, Hash, InfoResponse, Order, QueryRequest, QueryResponse, StdResult,
        Storage, WasmRawResponse, WasmSmartResponse,
    },
    cw_vm::{BackendQuerier, Instance, VmResult},
};

const DEFAULT_PAGE_LIMIT: u32 = 30;

// ------------------------------ backend querier ------------------------------

pub struct Querier<S> {
    store: S,
    block: BlockInfo,
}

impl<S> Querier<S> {
    pub fn new(store: S, block: BlockInfo) -> Self {
        Self { store, block }
    }
}

impl<S: Storage + Clone + 'static> BackendQuerier for Querier<S> {
    fn query_chain(&self, req: QueryRequest) -> VmResult<GenericResult<QueryResponse>> {
        Ok(process_query(self.store.clone(), &self.block, req).into())
    }
}

// ------------------------------- process query -------------------------------

pub fn process_query<S: Storage + Clone + 'static>(
    store: S,
    block: &BlockInfo,
    req:   QueryRequest,
) -> anyhow::Result<QueryResponse> {
    match req {
        QueryRequest::Info {} => query_info(&store).map(QueryResponse::Info),
        QueryRequest::Balance {
            address,
            denom,
        } => query_balance(store, block, address, denom).map(QueryResponse::Balance),
        QueryRequest::Balances {
            address,
            start_after,
            limit,
        } => query_balances(store, block, address, start_after, limit).map(QueryResponse::Balances),
        QueryRequest::Supply {
            denom,
        } => query_supply(store, block, denom).map(QueryResponse::Supply),
        QueryRequest::Supplies {
            start_after,
            limit,
        } => query_supplies(store, block, start_after, limit).map(QueryResponse::Supplies),
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

fn query_balance<S: Storage + Clone + 'static>(
    store:   S,
    block:   &BlockInfo,
    address: Addr,
    denom:   String,
) -> anyhow::Result<Coin> {
    _query_bank(store, block, &BankQuery::Balance { address, denom })
        .map(|res| res.as_balance())
}

fn query_balances<S: Storage + Clone + 'static>(
    store:       S,
    block:       &BlockInfo,
    address:     Addr,
    start_after: Option<String>,
    limit:       Option<u32>,
) -> anyhow::Result<Coins> {
    _query_bank(store, block, &BankQuery::Balances { address, start_after, limit })
        .map(|res| res.as_balances())
}

fn query_supply<S: Storage + Clone + 'static>(
    store: S,
    block: &BlockInfo,
    denom: String,
) -> anyhow::Result<Coin> {
    _query_bank(store, block, &BankQuery::Supply { denom })
        .map(|res| res.as_supply())
}

fn query_supplies<S: Storage + Clone + 'static>(
    store:       S,
    block:       &BlockInfo,
    start_after: Option<String>,
    limit:       Option<u32>,
) -> anyhow::Result<Coins> {
    _query_bank(store, block, &BankQuery::Supplies { start_after, limit })
        .map(|res| res.as_supplies())
}

fn _query_bank<S: Storage + Clone + 'static>(
    store: S,
    block: &BlockInfo,
    msg:   &BankQuery,
) -> anyhow::Result<BankQueryResponse> {
    // load wasm code
    let cfg = CONFIG.load(&store)?;
    let account = ACCOUNTS.load(&store, &cfg.bank)?;
    let wasm_byte_code = CODES.load(&store, &account.code_hash)?;

    // create wasm host
    let substore = PrefixStore::new(store.clone(), &[CONTRACT_NAMESPACE, &cfg.bank]);
    let querier = Querier::new(store, block.clone());
    let mut instance = Instance::build_from_code(substore, querier, &wasm_byte_code)?;

    // call query
    let ctx = Context {
        block:    block.clone(),
        contract: cfg.bank,
        sender:   None,
        funds:    None,
        simulate: None,
    };
    instance.call_query_bank(&ctx, msg)?.into_std_result().map_err(Into::into)
}

fn query_code(store: &dyn Storage, hash: Hash) -> anyhow::Result<Binary> {
    CODES.load(store, &hash).map_err(Into::into)
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
        .collect::<StdResult<Vec<_>>>()
        .map_err(Into::into)
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
    let substore = PrefixStore::new(store, &[CONTRACT_NAMESPACE, &contract]);
    let value = substore.read(&key)?;
    Ok(WasmRawResponse {
        contract,
        key,
        value: value.map(Binary::from),
    })
}

fn query_wasm_smart<S: Storage + Clone + 'static>(
    store:    S,
    block:    &BlockInfo,
    contract: Addr,
    msg:      Binary,
) -> anyhow::Result<WasmSmartResponse> {
    // load wasm code
    let account = ACCOUNTS.load(&store, &contract)?;
    let wasm_byte_code = CODES.load(&store, &account.code_hash)?;

    // create wasm host
    let substore = PrefixStore::new(store.clone(), &[CONTRACT_NAMESPACE, &contract]);
    let querier = Querier::new(store, block.clone());
    let mut instance = Instance::build_from_code(substore, querier, &wasm_byte_code)?;

    // call query
    let ctx = Context {
        block:    block.clone(),
        sender:   None,
        funds:    None,
        simulate: None,
        contract,
    };
    let data = instance.call_query(&ctx, msg)?.into_std_result()?;

    Ok(WasmSmartResponse {
        contract: ctx.contract,
        data,
    })
}
