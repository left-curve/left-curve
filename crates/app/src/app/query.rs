use {
    super::{ACCOUNTS, CODES, CONTRACT_NAMESPACE, LAST_FINALIZED_BLOCK},
    crate::wasm::must_build_wasm_instance,
    cw_std::{
        AccountResponse, Addr, Binary, BlockInfo, Bound, Context, Hash, InfoResponse, Order, Query,
        QueryResponse, Storage, WasmRawResponse, WasmSmartResponse,
    },
    cw_vm::Host,
};

const DEFAULT_PAGE_LIMIT: u32 = 30;

pub fn process_query<S: Storage + 'static>(
    store: S,
    block: &BlockInfo,
    req:   Query,
) -> (anyhow::Result<QueryResponse>, S) {
    match req {
        Query::Info {} => (query_info(&store).map(QueryResponse::Info), store),
        Query::Code {
            hash,
        } => (query_code(&store, hash).map(QueryResponse::Code), store),
        Query::Codes {
            start_after,
            limit,
        } => (query_codes(&store, start_after, limit).map(QueryResponse::Codes), store),
        Query::Account {
            address,
        } => (query_account(&store, address).map(QueryResponse::Account), store),
        Query::Accounts {
            start_after,
            limit,
        } => (query_accounts(&store, start_after, limit).map(QueryResponse::Accounts), store),
        Query::WasmRaw {
            contract,
            key,
        } => (query_wasm_raw(&store, contract, key).map(QueryResponse::WasmRaw), store),
        Query::WasmSmart {
            contract,
            msg
        } => {
            let (resp, store) = query_wasm_smart(store, block, contract, msg);
            (resp.map(QueryResponse::WasmSmart), store)
        },
    }
}

fn query_info(store: &dyn Storage) -> anyhow::Result<InfoResponse> {
    let block = LAST_FINALIZED_BLOCK.load(store)?;
    Ok(InfoResponse {
        chain_id:        block.chain_id,
        block_height:    block.height,
        block_timestamp: block.timestamp,
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

fn query_wasm_raw(
    _store:    &dyn Storage,
    _contract: Addr,
    _key:      Binary,
) -> anyhow::Result<WasmRawResponse> {
    todo!()
}

fn query_wasm_smart<S: Storage + 'static>(
    store:    S,
    block:    &BlockInfo,
    contract: Addr,
    msg:      Binary,
) -> (anyhow::Result<WasmSmartResponse>, S) {
    // load contract info
    let account = match ACCOUNTS.load(&store, &contract) {
        Ok(account) => account,
        Err(err) => return (Err(err), store),
    };

    // load wasm code
    let wasm_byte_code = match CODES.load(&store, &account.code_hash) {
        Ok(wasm_byte_code) => wasm_byte_code,
        Err(err) => return (Err(err), store),
    };

    // create wasm host
    let (instance, mut wasm_store) = must_build_wasm_instance(
        store,
        CONTRACT_NAMESPACE,
        &contract,
        wasm_byte_code,
    );
    let mut host = Host::new(&instance, &mut wasm_store);

    // call query
    let ctx = Context {
        block:    block.clone(),
        sender:   None,
        simulate: None,
        contract,
    };
    let data = match host.call_query(&ctx, msg) {
        Ok(data) => data,
        Err(err) => {
            let store = wasm_store.into_data().disassemble();
            return (Err(err), store);
        },
    };

    let query_res = WasmSmartResponse {
        contract: ctx.contract,
        data,
    };

    (Ok(query_res), wasm_store.into_data().disassemble())
}
