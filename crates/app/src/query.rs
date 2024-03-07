use {
    crate::{
        AppResult, Querier, ACCOUNTS, CHAIN_ID, CODES, CONFIG, CONTRACT_NAMESPACE,
        LAST_FINALIZED_BLOCK,
    },
    cw_db::PrefixStore,
    cw_std::{
        AccountResponse, Addr, BankQueryMsg, BankQueryResponse, Binary, BlockInfo, Bound, Coin,
        Coins, Context, Hash, InfoResponse, Order, StdResult, Storage, WasmRawResponse,
        WasmSmartResponse,
    },
    cw_vm::{BackendStorage, Instance},
};

const DEFAULT_PAGE_LIMIT: u32 = 30;

pub fn query_info(store: &dyn Storage) -> AppResult<InfoResponse> {
    Ok(InfoResponse {
        chain_id:             CHAIN_ID.load(store)?,
        config:               CONFIG.load(store)?,
        last_finalized_block: LAST_FINALIZED_BLOCK.load(store)?,
    })
}

pub fn query_balance<S: Storage + Clone + 'static>(
    store:   S,
    block:   &BlockInfo,
    address: Addr,
    denom:   String,
) -> AppResult<Coin> {
    _query_bank(store, block, &BankQueryMsg::Balance { address, denom })
        .map(|res| res.as_balance())
}

pub fn query_balances<S: Storage + Clone + 'static>(
    store:       S,
    block:       &BlockInfo,
    address:     Addr,
    start_after: Option<String>,
    limit:       Option<u32>,
) -> AppResult<Coins> {
    _query_bank(store, block, &BankQueryMsg::Balances { address, start_after, limit })
        .map(|res| res.as_balances())
}

pub fn query_supply<S: Storage + Clone + 'static>(
    store: S,
    block: &BlockInfo,
    denom: String,
) -> AppResult<Coin> {
    _query_bank(store, block, &BankQueryMsg::Supply { denom })
        .map(|res| res.as_supply())
}

pub fn query_supplies<S: Storage + Clone + 'static>(
    store:       S,
    block:       &BlockInfo,
    start_after: Option<String>,
    limit:       Option<u32>,
) -> AppResult<Coins> {
    _query_bank(store, block, &BankQueryMsg::Supplies { start_after, limit })
        .map(|res| res.as_supplies())
}

pub fn _query_bank<S: Storage + Clone + 'static>(
    store: S,
    block: &BlockInfo,
    msg:   &BankQueryMsg,
) -> AppResult<BankQueryResponse> {
    // load wasm code
    let chain_id = CHAIN_ID.load(&store)?;
    let cfg = CONFIG.load(&store)?;
    let account = ACCOUNTS.load(&store, &cfg.bank)?;
    let wasm_byte_code = CODES.load(&store, &account.code_hash)?;

    // create wasm host
    let substore = PrefixStore::new(store.clone(), &[CONTRACT_NAMESPACE, &cfg.bank]);
    let querier = Querier::new(store, block.clone());
    let mut instance = Instance::build_from_code(substore, querier, &wasm_byte_code)?;

    // call query
    let ctx = Context {
        chain_id,
        block_height:    block.height,
        block_timestamp: block.timestamp,
        block_hash:      block.hash.clone(),
        contract:        cfg.bank,
        sender:          None,
        funds:           None,
        simulate:        None,
    };
    instance.call_bank_query(&ctx, msg)?.into_std_result().map_err(Into::into)
}

pub fn query_code(store: &dyn Storage, hash: Hash) -> AppResult<Binary> {
    CODES.load(store, &hash).map_err(Into::into)
}

pub fn query_codes(
    store:       &dyn Storage,
    start_after: Option<Hash>,
    limit:       Option<u32>,
) -> AppResult<Vec<Hash>> {
    let start = start_after.as_ref().map(Bound::exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    CODES
        .keys(store, start, None, Order::Ascending)
        .take(limit as usize)
        .collect::<StdResult<Vec<_>>>()
        .map_err(Into::into)
}

pub fn query_account(store: &dyn Storage, address: Addr) -> AppResult<AccountResponse> {
    let account = ACCOUNTS.load(store, &address)?;
    Ok(AccountResponse {
        address,
        code_hash: account.code_hash,
        admin:     account.admin,
    })
}

pub fn query_accounts(
    store:       &dyn Storage,
    start_after: Option<Addr>,
    limit:       Option<u32>,
) -> AppResult<Vec<AccountResponse>> {
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

pub fn query_wasm_raw<S: Storage + 'static>(
    store:    S,
    contract: Addr,
    key:      Binary,
) -> AppResult<WasmRawResponse> {
    let substore = PrefixStore::new(store, &[CONTRACT_NAMESPACE, &contract]);
    let value = substore.read(&key)?;
    Ok(WasmRawResponse {
        contract,
        key,
        value: value.map(Binary::from),
    })
}

pub fn query_wasm_smart<S: Storage + Clone + 'static>(
    store:    S,
    block:    &BlockInfo,
    contract: Addr,
    msg:      Binary,
) -> AppResult<WasmSmartResponse> {
    // load wasm code
    let chain_id = CHAIN_ID.load(&store)?;
    let account = ACCOUNTS.load(&store, &contract)?;
    let wasm_byte_code = CODES.load(&store, &account.code_hash)?;

    // create wasm host
    let substore = PrefixStore::new(store.clone(), &[CONTRACT_NAMESPACE, &contract]);
    let querier = Querier::new(store, block.clone());
    let mut instance = Instance::build_from_code(substore, querier, &wasm_byte_code)?;

    // call query
    let ctx = Context {
        chain_id,
        contract,
        block_height:    block.height,
        block_timestamp: block.timestamp,
        block_hash:      block.hash.clone(),
        sender:          None,
        funds:           None,
        simulate:        None,
    };
    let data = instance.call_query(&ctx, msg)?.into_std_result()?;

    Ok(WasmSmartResponse {
        contract: ctx.contract,
        data,
    })
}
