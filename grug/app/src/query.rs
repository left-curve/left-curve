use {
    crate::{
        call_in_1_out_1, AppError, AppResult, GasTracker, MeteredItem, MeteredMap, MeteredStorage,
        StorageProvider, Vm, APP_CONFIGS, CHAIN_ID, CODES, CONFIG, CONTRACTS, CONTRACT_NAMESPACE,
    },
    grug_types::{
        Addr, BankQuery, BankQueryResponse, Binary, BlockInfo, Bound, Coin, Coins, Config, Context,
        ContractInfo, Denom, GenericResult, Hash256, Json, Order, StdResult, Storage,
    },
    std::collections::BTreeMap,
};

const DEFAULT_PAGE_LIMIT: u32 = 30;

pub fn query_config(storage: &dyn Storage, gas_tracker: GasTracker) -> StdResult<Config> {
    CONFIG.load_with_gas(storage, gas_tracker)
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
    gas_tracker: GasTracker,
    query_depth: usize,
    block: BlockInfo,
    address: Addr,
    denom: Denom,
) -> AppResult<Coin>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    _query_bank(
        vm,
        storage,
        gas_tracker,
        query_depth,
        block,
        &BankQuery::Balance { address, denom },
    )
    .map(|res| res.as_balance())
}

pub fn query_balances<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    query_depth: usize,
    block: BlockInfo,
    address: Addr,
    start_after: Option<Denom>,
    limit: Option<u32>,
) -> AppResult<Coins>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    _query_bank(
        vm,
        storage,
        gas_tracker,
        query_depth,
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
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    query_depth: usize,
    block: BlockInfo,
    denom: Denom,
) -> AppResult<Coin>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    _query_bank(
        vm,
        storage,
        gas_tracker,
        query_depth,
        block,
        &BankQuery::Supply { denom },
    )
    .map(|res| res.as_supply())
}

pub fn query_supplies<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    query_depth: usize,
    block: BlockInfo,
    start_after: Option<Denom>,
    limit: Option<u32>,
) -> AppResult<Coins>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    _query_bank(
        vm,
        storage,
        gas_tracker,
        query_depth,
        block,
        &BankQuery::Supplies { start_after, limit },
    )
    .map(|res| res.as_supplies())
}

fn _query_bank<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    query_depth: usize,
    block: BlockInfo,
    msg: &BankQuery,
) -> AppResult<BankQueryResponse>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let chain_id = CHAIN_ID.load(&storage)?;
    let cfg = CONFIG.load(&storage)?;
    let code_hash = CONTRACTS.load(&storage, cfg.bank)?.code_hash;

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
        query_depth,
        false,
        "bank_query",
        code_hash,
        &ctx,
        msg,
    )?
    .map_err(|msg| AppError::Guest {
        address: ctx.contract,
        name: "bank_query",
        msg,
    })
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
    let start = start_after.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    CODES
        .range_with_gas(storage, gas_tracker, start, None, Order::Ascending)?
        .take(limit as usize)
        .collect()
}

pub fn query_contract(
    storage: &dyn Storage,
    gas_tracker: GasTracker,
    address: Addr,
) -> StdResult<ContractInfo> {
    CONTRACTS.load_with_gas(storage, gas_tracker, address)
}

pub fn query_contracts(
    storage: &dyn Storage,
    gas_tracker: GasTracker,
    start_after: Option<Addr>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<Addr, ContractInfo>> {
    let start = start_after.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    CONTRACTS
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
    gas_tracker: GasTracker,
    query_depth: usize,
    block: BlockInfo,
    contract: Addr,
    msg: Json,
) -> AppResult<Json>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let chain_id = CHAIN_ID.load(&storage)?;
    let code_hash = CONTRACTS.load(&storage, contract)?.code_hash;

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
        query_depth,
        false,
        "query",
        code_hash,
        &ctx,
        &msg,
    )?
    .map_err(|msg| AppError::Guest {
        address: ctx.contract,
        name: "query",
        msg,
    })
}
