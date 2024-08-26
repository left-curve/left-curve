use {
    crate::{
        call_in_1_out_1, AppError, AppResult, GasTracker, MeteredItem, MeteredMap, MeteredStorage,
        StorageProvider, Vm, ACCOUNTS, APP_CONFIGS, CHAIN_ID, CODES, CONFIG, CONTRACT_NAMESPACE,
        LAST_FINALIZED_BLOCK,
    },
    grug_storage::Bound,
    grug_types::{
        Account, Addr, BankQuery, BankQueryResponse, Binary, BlockInfo, Coin, Coins, Context,
        GenericResult, Hash256, InfoResponse, Json, Order, StdResult, Storage,
    },
    std::collections::BTreeMap,
};

const DEFAULT_PAGE_LIMIT: u32 = 30;

pub fn query_info(storage: &dyn Storage, gas_tracker: GasTracker) -> StdResult<InfoResponse> {
    Ok(InfoResponse {
        chain_id: CHAIN_ID.load_with_gas(storage, gas_tracker.clone())?,
        config: CONFIG.load_with_gas(storage, gas_tracker.clone())?,
        last_finalized_block: LAST_FINALIZED_BLOCK.load_with_gas(storage, gas_tracker)?,
    })
}

pub fn query_app_config(
    storage: &dyn Storage,
    gas_tracker: GasTracker,
    key: &str,
) -> StdResult<Json> {
    APP_CONFIGS.load_with_gas(storage, gas_tracker, key)
}

pub fn query_app_configs(
    storage: &dyn Storage,
    gas_tracker: GasTracker,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<String, Json>> {
    let start = start_after.as_deref().map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    APP_CONFIGS
        .range_with_gas(storage, gas_tracker, start, None, Order::Ascending)?
        .take(limit)
        .collect()
}

pub fn query_balance<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    block: BlockInfo,
    gas_tracker: GasTracker,
    query_depth: usize,
    address: Addr,
    denom: String,
) -> AppResult<Coin>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    _query_bank(
        vm,
        storage,
        block,
        gas_tracker,
        query_depth,
        &BankQuery::Balance { address, denom },
    )
    .map(|res| res.as_balance())
}

pub fn query_balances<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    block: BlockInfo,
    gas_tracker: GasTracker,
    query_depth: usize,
    address: Addr,
    start_after: Option<String>,
    limit: Option<u32>,
) -> AppResult<Coins>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    _query_bank(
        vm,
        storage,
        block,
        gas_tracker,
        query_depth,
        &BankQuery::Balances {
            address,
            start_after,
            limit,
        },
    )
    .map(|res| res.as_balances())
}

pub fn query_supply<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    block: BlockInfo,
    gas_tracker: GasTracker,
    query_depth: usize,
    denom: String,
) -> AppResult<Coin>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    _query_bank(
        vm,
        storage,
        block,
        gas_tracker,
        query_depth,
        &BankQuery::Supply { denom },
    )
    .map(|res| res.as_supply())
}

pub fn query_supplies<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    block: BlockInfo,
    gas_tracker: GasTracker,
    query_depth: usize,
    start_after: Option<String>,
    limit: Option<u32>,
) -> AppResult<Coins>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    _query_bank(
        vm,
        storage,
        block,
        gas_tracker,
        query_depth,
        &BankQuery::Supplies { start_after, limit },
    )
    .map(|res| res.as_supplies())
}

fn _query_bank<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    block: BlockInfo,
    gas_tracker: GasTracker,
    query_depth: usize,
    msg: &BankQuery,
) -> AppResult<BankQueryResponse>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let chain_id = CHAIN_ID.load(&storage)?;
    let cfg = CONFIG.load(&storage)?;
    let account = ACCOUNTS.load(&storage, cfg.bank)?;

    let ctx = Context {
        chain_id,
        block,
        contract: cfg.bank,
        sender: None,
        funds: None,
        mode: None,
    };

    call_in_1_out_1::<_, _, GenericResult<BankQueryResponse>>(
        vm,
        storage,
        gas_tracker,
        "bank_query",
        account.code_hash,
        &ctx,
        true,
        query_depth,
        msg,
    )?
    .into_std_result()
    .map_err(AppError::Std)
}

pub fn query_code(
    storage: &dyn Storage,
    gas_tracker: GasTracker,
    hash: Hash256,
) -> StdResult<Binary> {
    CODES.load_with_gas(storage, gas_tracker, hash)
}

pub fn query_codes(
    storage: &dyn Storage,
    gas_tracker: GasTracker,
    start_after: Option<Hash256>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<Hash256, Binary>> {
    let start = start_after.map(Bound::exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    CODES
        .range_with_gas(storage, gas_tracker, start, None, Order::Ascending)?
        .take(limit as usize)
        .collect()
}

pub fn query_account(
    storage: &dyn Storage,
    gas_tracker: GasTracker,
    address: Addr,
) -> StdResult<Account> {
    ACCOUNTS.load_with_gas(storage, gas_tracker, address)
}

pub fn query_accounts(
    storage: &dyn Storage,
    gas_tracker: GasTracker,
    start_after: Option<Addr>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<Addr, Account>> {
    let start = start_after.map(Bound::exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    ACCOUNTS
        .range_with_gas(storage, gas_tracker, start, None, Order::Ascending)?
        .take(limit as usize)
        .collect()
}

pub fn query_wasm_raw(
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    contract: Addr,
    key: Binary,
) -> StdResult<Option<Binary>> {
    StorageProvider::new(storage, &[CONTRACT_NAMESPACE, &contract])
        .read_with_gas(gas_tracker, &key)
        .map(|maybe_value| maybe_value.map(Binary::from))
}

pub fn query_wasm_smart<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    block: BlockInfo,
    gas_tracker: GasTracker,
    query_depth: usize,
    contract: Addr,
    msg: Json,
) -> AppResult<Json>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let chain_id = CHAIN_ID.load(&storage)?;
    let account = ACCOUNTS.load(&storage, contract)?;

    let ctx = Context {
        chain_id,
        block,
        contract,
        sender: None,
        funds: None,
        mode: None,
    };

    call_in_1_out_1::<_, _, GenericResult<Json>>(
        vm,
        storage,
        gas_tracker,
        "query",
        account.code_hash,
        &ctx,
        true,
        query_depth,
        &msg,
    )?
    .into_std_result()
    .map_err(AppError::Std)
}
