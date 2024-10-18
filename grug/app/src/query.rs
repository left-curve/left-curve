use {
    crate::{
        call_in_1_out_1, AppCtx, AppError, AppResult, MeteredItem, MeteredMap, MeteredStorage,
        StorageProvider, Vm, APP_CONFIGS, CHAIN_ID, CODES, CONFIG, CONTRACTS, CONTRACT_NAMESPACE,
    },
    grug_types::{
        Addr, BankQuery, BankQueryResponse, Binary, Bound, Coin, Coins, Config, Context,
        ContractInfo, GenericResult, Hash256, Json, Order, QueryAppConfigRequest,
        QueryAppConfigsRequest, QueryBalanceRequest, QueryBalancesRequest, QueryCodeRequest,
        QueryCodesRequest, QueryContractRequest, QueryContractsRequest, QuerySuppliesRequest,
        QuerySupplyRequest, QueryWasmRawRequest, QueryWasmSmartRequest, StdResult,
    },
    std::collections::BTreeMap,
};

const DEFAULT_PAGE_LIMIT: u32 = 30;

pub fn query_config(ctx: AppCtx) -> StdResult<Config> {
    CONFIG.load_with_gas(&ctx.storage, ctx.gas_tracker)
}

pub fn query_app_config(ctx: AppCtx, req: QueryAppConfigRequest) -> StdResult<Json> {
    APP_CONFIGS.load_with_gas(&ctx.storage, ctx.gas_tracker, &req.key)
}

pub fn query_app_configs(
    ctx: AppCtx,
    req: QueryAppConfigsRequest,
) -> StdResult<BTreeMap<String, Json>> {
    let start = req.start_after.as_deref().map(Bound::Exclusive);
    let limit = req.limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    APP_CONFIGS
        .range_with_gas(&ctx.storage, ctx.gas_tracker, start, None, Order::Ascending)?
        .take(limit)
        .collect()
}

pub fn query_balance<VM>(
    ctx: AppCtx<VM>,
    query_depth: usize,
    req: QueryBalanceRequest,
) -> AppResult<Coin>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    _query_bank(ctx, query_depth, &BankQuery::Balance(req)).map(|res| res.as_balance())
}

pub fn query_balances<VM>(
    ctx: AppCtx<VM>,
    query_depth: usize,
    req: QueryBalancesRequest,
) -> AppResult<Coins>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    _query_bank(ctx, query_depth, &BankQuery::Balances(req)).map(|res| res.as_balances())
}

pub fn query_supply<VM>(
    ctx: AppCtx<VM>,
    query_depth: usize,
    req: QuerySupplyRequest,
) -> AppResult<Coin>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    _query_bank(ctx, query_depth, &BankQuery::Supply(req)).map(|res| res.as_supply())
}

pub fn query_supplies<VM>(
    ctx: AppCtx<VM>,
    query_depth: usize,
    req: QuerySuppliesRequest,
) -> AppResult<Coins>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    _query_bank(ctx, query_depth, &BankQuery::Supplies(req)).map(|res| res.as_supplies())
}

fn _query_bank<VM>(
    app_ctx: AppCtx<VM>,
    query_depth: usize,
    msg: &BankQuery,
) -> AppResult<BankQueryResponse>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let chain_id = CHAIN_ID.load(&app_ctx.storage)?;
    let cfg = CONFIG.load(&app_ctx.storage)?;
    let code_hash = CONTRACTS.load(&app_ctx.storage, cfg.bank)?.code_hash;

    let ctx = Context {
        chain_id,
        block: app_ctx.block,
        contract: cfg.bank,
        sender: None,
        funds: None,
        mode: None,
    };

    call_in_1_out_1::<_, _, GenericResult<BankQueryResponse>>(
        app_ctx,
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

pub fn query_code(ctx: AppCtx, req: QueryCodeRequest) -> StdResult<Binary> {
    CODES.load_with_gas(&ctx.storage, ctx.gas_tracker, req.hash)
}

pub fn query_codes(ctx: AppCtx, req: QueryCodesRequest) -> StdResult<BTreeMap<Hash256, Binary>> {
    let start = req.start_after.map(Bound::Exclusive);
    let limit = req.limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    CODES
        .range_with_gas(&ctx.storage, ctx.gas_tracker, start, None, Order::Ascending)?
        .take(limit as usize)
        .collect()
}

pub fn query_contract(ctx: AppCtx, req: QueryContractRequest) -> StdResult<ContractInfo> {
    CONTRACTS.load_with_gas(&ctx.storage, ctx.gas_tracker, req.address)
}

pub fn query_contracts(
    ctx: AppCtx,
    req: QueryContractsRequest,
) -> StdResult<BTreeMap<Addr, ContractInfo>> {
    let start = req.start_after.map(Bound::Exclusive);
    let limit = req.limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    CONTRACTS
        .range_with_gas(&ctx.storage, ctx.gas_tracker, start, None, Order::Ascending)?
        .take(limit as usize)
        .collect()
}

pub fn query_wasm_raw(ctx: AppCtx, req: QueryWasmRawRequest) -> StdResult<Option<Binary>> {
    StorageProvider::new(ctx.storage, &[CONTRACT_NAMESPACE, &req.contract])
        .read_with_gas(ctx.gas_tracker, &req.key)
        .map(|maybe_value| maybe_value.map(Binary::from))
}

pub fn query_wasm_smart<VM>(
    app_ctx: AppCtx<VM>,
    query_depth: usize,
    req: QueryWasmSmartRequest,
) -> AppResult<Json>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let chain_id = CHAIN_ID.load(&app_ctx.storage)?;
    let code_hash = CONTRACTS.load(&app_ctx.storage, req.contract)?.code_hash;

    let ctx = Context {
        chain_id,
        block: app_ctx.block,
        contract: req.contract,
        sender: None,
        funds: None,
        mode: None,
    };

    call_in_1_out_1::<_, _, GenericResult<Json>>(
        app_ctx,
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
