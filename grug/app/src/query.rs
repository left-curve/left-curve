use {
    crate::{
        APP_CONFIG, AppError, AppResult, CHAIN_ID, CODES, CONFIG, CONTRACT_NAMESPACE, CONTRACTS,
        GasTracker, LAST_FINALIZED_BLOCK, MeteredItem, MeteredMap, MeteredStorage, StorageProvider,
        Vm, call_in_1_out_1,
    },
    grug_types::{
        Addr, BankQuery, BankQueryResponse, Binary, BlockInfo, Bound, Code, Coin, Coins, Config,
        Context, ContractInfo, DEFAULT_PAGE_LIMIT, GenericResult, Hash256, Json, Order,
        QueryBalanceRequest, QueryBalancesRequest, QueryCodeRequest, QueryCodesRequest,
        QueryContractRequest, QueryContractsRequest, QueryStatusResponse, QuerySuppliesRequest,
        QuerySupplyRequest, QueryWasmRawRequest, QueryWasmScanRequest, QueryWasmSmartRequest,
        StdResult, Storage,
    },
    std::collections::BTreeMap,
};

pub fn query_status(
    storage: &dyn Storage,
    gas_tracker: GasTracker,
) -> StdResult<QueryStatusResponse> {
    let chain_id = CHAIN_ID.load_with_gas(storage, gas_tracker.clone())?;
    let last_finalized_block = LAST_FINALIZED_BLOCK.load_with_gas(storage, gas_tracker)?;

    Ok(QueryStatusResponse {
        chain_id,
        last_finalized_block,
    })
}

pub fn query_config(storage: &dyn Storage, gas_tracker: GasTracker) -> StdResult<Config> {
    CONFIG.load_with_gas(storage, gas_tracker)
}

pub fn query_app_config(storage: &dyn Storage, gas_tracker: GasTracker) -> StdResult<Json> {
    APP_CONFIG.load_with_gas(storage, gas_tracker)
}

pub fn query_balance<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    query_depth: usize,
    req: QueryBalanceRequest,
) -> AppResult<Coin>
where
    VM: Vm + Clone + Send + Sync + 'static,
    AppError: From<VM::Error>,
{
    _query_bank(
        vm,
        storage,
        gas_tracker,
        block,
        query_depth,
        &BankQuery::Balance(req),
    )
    .map(|res| res.as_balance())
}

pub fn query_balances<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    query_depth: usize,
    req: QueryBalancesRequest,
) -> AppResult<Coins>
where
    VM: Vm + Clone + Send + Sync + 'static,
    AppError: From<VM::Error>,
{
    _query_bank(
        vm,
        storage,
        gas_tracker,
        block,
        query_depth,
        &BankQuery::Balances(req),
    )
    .map(|res| res.as_balances())
}

pub fn query_supply<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    query_depth: usize,
    req: QuerySupplyRequest,
) -> AppResult<Coin>
where
    VM: Vm + Clone + Send + Sync + 'static,
    AppError: From<VM::Error>,
{
    _query_bank(
        vm,
        storage,
        gas_tracker,
        block,
        query_depth,
        &BankQuery::Supply(req),
    )
    .map(|res| res.as_supply())
}

pub fn query_supplies<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    query_depth: usize,
    req: QuerySuppliesRequest,
) -> AppResult<Coins>
where
    VM: Vm + Clone + Send + Sync + 'static,
    AppError: From<VM::Error>,
{
    _query_bank(
        vm,
        storage,
        gas_tracker,
        block,
        query_depth,
        &BankQuery::Supplies(req),
    )
    .map(|res| res.as_supplies())
}

fn _query_bank<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    query_depth: usize,
    msg: &BankQuery,
) -> AppResult<BankQueryResponse>
where
    VM: Vm + Clone + Send + Sync + 'static,
    AppError: From<VM::Error>,
{
    let cfg = CONFIG.load(&storage)?;
    let chain_id = CHAIN_ID.load(&storage)?;
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
    block: BlockInfo,
    query_depth: usize,
    req: QueryWasmSmartRequest,
) -> AppResult<Json>
where
    VM: Vm + Clone + Send + Sync + 'static,
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
