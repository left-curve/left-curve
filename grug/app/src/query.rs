use {
    crate::{
        call_in_1_out_1, AppError, AppResult, GasTracker, MeteredItem, MeteredMap, MeteredStorage,
        StorageProvider, Vm, APP_CONFIGS, CHAIN_ID, CODES, CONFIG, CONTRACTS, CONTRACT_NAMESPACE,
    },
    grug_types::{
        Addr, BankQuery, BankQueryResponse, Binary, BlockInfo, Bound, Code, Coin, Coins, Config,
        Context, ContractInfo, GenericResult, Hash256, Json, Order, QueryAppConfigRequest,
        QueryAppConfigsRequest, QueryBalanceRequest, QueryBalancesRequest, QueryCodeRequest,
        QueryCodesRequest, QueryConfigRequest, QueryContractRequest, QueryContractsRequest,
        QuerySuppliesRequest, QuerySupplyRequest, QueryWasmRawRequest, QueryWasmScanRequest,
        QueryWasmSmartRequest, StdResult, Storage,
    },
    std::collections::BTreeMap,
};

const DEFAULT_PAGE_LIMIT: u32 = 30;

pub fn query_config(
    storage: &dyn Storage,
    gas_tracker: GasTracker,
    _req: QueryConfigRequest,
) -> StdResult<Config> {
    CONFIG.load_with_gas(storage, gas_tracker)
}

pub fn query_app_config(
    storage: &dyn Storage,
    gas_tracker: GasTracker,
    req: QueryAppConfigRequest,
) -> StdResult<Json> {
    APP_CONFIGS.load_with_gas(storage, gas_tracker, &req.key)
}

pub fn query_app_configs(
    storage: &dyn Storage,
    gas_tracker: GasTracker,
    req: QueryAppConfigsRequest,
) -> StdResult<BTreeMap<String, Json>> {
    let start = req.start_after.as_deref().map(Bound::Exclusive);
    let limit = req.limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

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
    req: QueryBalanceRequest,
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
        &BankQuery::Balance(req),
    )
    .map(|res| res.as_balance())
}

pub fn query_balances<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    query_depth: usize,
    block: BlockInfo,
    req: QueryBalancesRequest,
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
        &BankQuery::Balances(req),
    )
    .map(|res| res.as_balances())
}

pub fn query_supply<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    query_depth: usize,
    block: BlockInfo,
    req: QuerySupplyRequest,
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
        &BankQuery::Supply(req),
    )
    .map(|res| res.as_supply())
}

pub fn query_supplies<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    query_depth: usize,
    block: BlockInfo,
    req: QuerySuppliesRequest,
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
        &BankQuery::Supplies(req),
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
    req: QueryCodeRequest,
) -> StdResult<Code> {
    CODES.load_with_gas(storage, gas_tracker, req.hash)
}

pub fn query_codes(
    storage: &dyn Storage,
    gas_tracker: GasTracker,
    req: QueryCodesRequest,
) -> StdResult<BTreeMap<Hash256, Code>> {
    let start = req.start_after.map(Bound::Exclusive);
    let limit = req.limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    CODES
        .range_with_gas(storage, gas_tracker, start, None, Order::Ascending)?
        .take(limit as usize)
        .collect()
}

pub fn query_contract(
    storage: &dyn Storage,
    gas_tracker: GasTracker,
    req: QueryContractRequest,
) -> StdResult<ContractInfo> {
    CONTRACTS.load_with_gas(storage, gas_tracker, req.address)
}

pub fn query_contracts(
    storage: &dyn Storage,
    gas_tracker: GasTracker,
    req: QueryContractsRequest,
) -> StdResult<BTreeMap<Addr, ContractInfo>> {
    let start = req.start_after.map(Bound::Exclusive);
    let limit = req.limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    CONTRACTS
        .range_with_gas(storage, gas_tracker, start, None, Order::Ascending)?
        .take(limit as usize)
        .collect()
}

pub fn query_wasm_raw(
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    req: QueryWasmRawRequest,
) -> StdResult<Option<Binary>> {
    StorageProvider::new(storage, &[CONTRACT_NAMESPACE, &req.contract])
        .read_with_gas(gas_tracker, &req.key)
        .map(|maybe_value| maybe_value.map(Binary::from))
}

pub fn query_wasm_scan(
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    req: QueryWasmScanRequest,
) -> StdResult<BTreeMap<Binary, Binary>> {
    let limit = req.limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    StorageProvider::new(storage, &[CONTRACT_NAMESPACE, &req.contract])
        .scan_with_gas(
            gas_tracker,
            req.min.as_deref(),
            req.max.as_deref(),
            // Order doesn't matter, as we're collecting results into a BTreeMap.
            Order::Ascending,
        )?
        .take(limit as usize)
        .map(|res| res.map(|(k, v)| (Binary::from(k), Binary::from(v))))
        .collect()
}

pub fn query_wasm_smart<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    query_depth: usize,
    block: BlockInfo,
    req: QueryWasmSmartRequest,
) -> AppResult<Json>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let chain_id = CHAIN_ID.load(&storage)?;
    let code_hash = CONTRACTS.load(&storage, req.contract)?.code_hash;

    let ctx = Context {
        chain_id,
        block,
        contract: req.contract,
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
        &req.msg,
    )?
    .map_err(|msg| AppError::Guest {
        address: ctx.contract,
        name: "query",
        msg,
    })
}
