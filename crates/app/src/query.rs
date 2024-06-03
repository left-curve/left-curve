use {
    crate::{
        create_vm_instance, load_program, AppError, AppResult, PrefixStore, Vm, ACCOUNTS, CHAIN_ID,
        CODES, CONFIG, CONTRACT_NAMESPACE, LAST_FINALIZED_BLOCK,
    },
    grug_storage::Bound,
    grug_types::{
        AccountResponse, Addr, BankQuery, BankQueryResponse, Binary, BlockInfo, Coin, Coins,
        Context, Hash, InfoResponse, Json, Order, StdResult, Storage, WasmRawResponse,
        WasmSmartResponse,
    },
};

const DEFAULT_PAGE_LIMIT: u32 = 30;

pub fn query_info(storage: &dyn Storage) -> AppResult<InfoResponse> {
    Ok(InfoResponse {
        chain_id: CHAIN_ID.load(storage)?,
        config: CONFIG.load(storage)?,
        last_finalized_block: LAST_FINALIZED_BLOCK.load(storage)?,
    })
}

pub fn query_balance<VM>(
    storage: Box<dyn Storage>,
    block: &BlockInfo,
    address: Addr,
    denom: String,
) -> AppResult<Coin>
where
    VM: Vm,
    AppError: From<VM::Error>,
{
    _query_bank::<VM>(storage, block, &BankQuery::Balance { address, denom })
        .map(|res| res.as_balance())
}

pub fn query_balances<VM>(
    storage: Box<dyn Storage>,
    block: &BlockInfo,
    address: Addr,
    start_after: Option<String>,
    limit: Option<u32>,
) -> AppResult<Coins>
where
    VM: Vm,
    AppError: From<VM::Error>,
{
    _query_bank::<VM>(
        storage,
        block,
        &BankQuery::Balances {
            address,
            start_after,
            limit,
        },
    )
    .map(|res| res.as_balances())
}

pub fn query_supply<VM>(
    storage: Box<dyn Storage>,
    block: &BlockInfo,
    denom: String,
) -> AppResult<Coin>
where
    VM: Vm,
    AppError: From<VM::Error>,
{
    _query_bank::<VM>(storage, block, &BankQuery::Supply { denom }).map(|res| res.as_supply())
}

pub fn query_supplies<VM>(
    storage: Box<dyn Storage>,
    block: &BlockInfo,
    start_after: Option<String>,
    limit: Option<u32>,
) -> AppResult<Coins>
where
    VM: Vm,
    AppError: From<VM::Error>,
{
    _query_bank::<VM>(storage, block, &BankQuery::Supplies { start_after, limit })
        .map(|res| res.as_supplies())
}

pub fn _query_bank<VM>(
    storage: Box<dyn Storage>,
    block: &BlockInfo,
    msg: &BankQuery,
) -> AppResult<BankQueryResponse>
where
    VM: Vm,
    AppError: From<VM::Error>,
{
    // load program code
    let chain_id = CHAIN_ID.load(&storage)?;
    let cfg = CONFIG.load(&storage)?;
    let account = ACCOUNTS.load(&storage, &cfg.bank)?;

    // create VM instance
    let program = load_program::<VM>(&storage, &account.code_hash)?;
    let instance = create_vm_instance::<VM>(storage.clone(), block.clone(), &cfg.bank, program)?;

    // call query
    let ctx = Context {
        chain_id,
        block_height: block.height,
        block_timestamp: block.timestamp,
        block_hash: block.hash.clone(),
        contract: cfg.bank,
        sender: None,
        funds: None,
        simulate: None,
    };
    instance
        .call_bank_query(&ctx, msg)?
        .into_std_result()
        .map_err(Into::into)
}

pub fn query_code(storage: &dyn Storage, hash: Hash) -> AppResult<Binary> {
    Ok(CODES.load(storage, &hash)?.into())
}

pub fn query_codes(
    storage: &dyn Storage,
    start_after: Option<Hash>,
    limit: Option<u32>,
) -> AppResult<Vec<Hash>> {
    let start = start_after.as_ref().map(Bound::exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    CODES
        .keys(storage, start, None, Order::Ascending)
        .take(limit as usize)
        .collect::<StdResult<Vec<_>>>()
        .map_err(Into::into)
}

pub fn query_account(storage: &dyn Storage, address: Addr) -> AppResult<AccountResponse> {
    let account = ACCOUNTS.load(storage, &address)?;
    Ok(AccountResponse {
        address,
        code_hash: account.code_hash,
        admin: account.admin,
    })
}

pub fn query_accounts(
    storage: &dyn Storage,
    start_after: Option<Addr>,
    limit: Option<u32>,
) -> AppResult<Vec<AccountResponse>> {
    let start = start_after.as_ref().map(Bound::exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    ACCOUNTS
        .range(storage, start, None, Order::Ascending)
        .take(limit as usize)
        .map(|item| {
            let (address, account) = item?;
            Ok(AccountResponse {
                address,
                code_hash: account.code_hash,
                admin: account.admin,
            })
        })
        .collect()
}

pub fn query_wasm_raw(
    storage: Box<dyn Storage>,
    contract: Addr,
    key: Binary,
) -> AppResult<WasmRawResponse> {
    let substore = PrefixStore::new(storage, &[CONTRACT_NAMESPACE, &contract]);
    let value = substore.read(&key);
    Ok(WasmRawResponse {
        contract,
        key,
        value: value.map(Binary::from),
    })
}

pub fn query_wasm_smart<VM>(
    storage: Box<dyn Storage>,
    block: &BlockInfo,
    contract: Addr,
    msg: Json,
) -> AppResult<WasmSmartResponse>
where
    VM: Vm,
    AppError: From<VM::Error>,
{
    let chain_id = CHAIN_ID.load(&storage)?;
    let account = ACCOUNTS.load(&storage, &contract)?;

    let program = load_program::<VM>(&storage, &account.code_hash)?;
    let instance = create_vm_instance::<VM>(storage, block.clone(), &contract, program)?;

    // call query
    let ctx = Context {
        chain_id,
        contract,
        block_height: block.height,
        block_timestamp: block.timestamp,
        block_hash: block.hash.clone(),
        sender: None,
        funds: None,
        simulate: None,
    };
    let data = instance.call_query(&ctx, &msg)?.into_std_result()?;

    Ok(WasmSmartResponse {
        contract: ctx.contract,
        data,
    })
}
