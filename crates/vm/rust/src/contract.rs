use {
    crate::{
        AfterTxFn, BankExecuteFn, BankQueryFn, BeforeTxFn, Contract, CronExecuteFn, ExecuteFn,
        HandleFeeFn, InstantiateFn, MigrateFn, QueryFn, ReceiveFn, ReplyFn, VmError, VmResult,
    },
    elsa::sync::FrozenVec,
    grug_types::{
        from_json_slice, make_auth_ctx, make_immutable_ctx, make_mutable_ctx, make_sudo_ctx, Api,
        AuthCtx, BankMsg, BankQuery, BankQueryResponse, Binary, Context, Empty, GenericResult,
        ImmutableCtx, Json, MutableCtx, Outcome, Querier, QuerierWrapper, Response, StdError,
        Storage, SubMsgResult, SudoCtx, Tx,
    },
    serde::de::DeserializeOwned,
    std::sync::OnceLock,
};

static CONTRACTS: OnceLock<FrozenVec<Box<dyn Contract + Send + Sync>>> = OnceLock::new();

pub(crate) fn get_contract_impl(
    wrapper: ContractWrapper,
) -> VmResult<&'static (dyn Contract + Send + Sync)> {
    CONTRACTS
        .get_or_init(Default::default)
        .get(wrapper.index)
        .ok_or_else(|| VmError::contract_not_found(wrapper.index))
}

// ---------------------------------- wrapper ----------------------------------

#[derive(Debug, Clone, Copy)]
pub struct ContractWrapper {
    index: usize,
}

impl<T> From<T> for ContractWrapper
where
    T: AsRef<[u8]>,
{
    fn from(bytes: T) -> Self {
        Self {
            index: usize::from_le_bytes(bytes.as_ref().try_into().unwrap()),
        }
    }
}

impl From<ContractWrapper> for Binary {
    fn from(wrapper: ContractWrapper) -> Self {
        wrapper.index.to_le_bytes().into()
    }
}

// ---------------------------------- builder ----------------------------------

pub struct ContractBuilder<
    M1,
    E1,
    M2 = Empty,
    M3 = Empty,
    M5 = Empty,
    M6 = Empty,
    E2 = StdError,
    E3 = StdError,
    E4 = StdError,
    E5 = StdError,
    E6 = StdError,
    E7 = StdError,
    E8 = StdError,
    E9 = StdError,
    E10 = StdError,
    E11 = StdError,
    E12 = StdError,
> {
    instantiate_fn: InstantiateFn<M1, E1>,
    execute_fn: Option<ExecuteFn<M2, E2>>,
    migrate_fn: Option<MigrateFn<M3, E3>>,
    receive_fn: Option<ReceiveFn<E4>>,
    reply_fn: Option<ReplyFn<M5, E5>>,
    query_fn: Option<QueryFn<M6, E6>>,
    before_tx_fn: Option<BeforeTxFn<E7>>,
    after_tx_fn: Option<AfterTxFn<E8>>,
    handle_fee_fn: Option<HandleFeeFn<E9>>,
    bank_execute_fn: Option<BankExecuteFn<E10>>,
    bank_query_fn: Option<BankQueryFn<E11>>,
    cron_execute_fn: Option<CronExecuteFn<E12>>,
}

impl<M1, E1> ContractBuilder<M1, E1>
where
    M1: DeserializeOwned + 'static,
    E1: ToString + 'static,
{
    pub fn new(instantiate_fn: InstantiateFn<M1, E1>) -> Self {
        Self {
            instantiate_fn,
            execute_fn: None,
            migrate_fn: None,
            receive_fn: None,
            reply_fn: None,
            query_fn: None,
            before_tx_fn: None,
            after_tx_fn: None,
            handle_fee_fn: None,
            bank_execute_fn: None,
            bank_query_fn: None,
            cron_execute_fn: None,
        }
    }
}

impl<M1, E1, M2, M3, M5, M6, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11, E12>
    ContractBuilder<M1, E1, M2, M3, M5, M6, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11, E12>
where
    M1: DeserializeOwned + 'static,
    M2: DeserializeOwned + 'static,
    M3: DeserializeOwned + 'static,
    M5: DeserializeOwned + 'static,
    M6: DeserializeOwned + 'static,
    E1: ToString + 'static,
    E2: ToString + 'static,
    E3: ToString + 'static,
    E4: ToString + 'static,
    E5: ToString + 'static,
    E6: ToString + 'static,
    E7: ToString + 'static,
    E8: ToString + 'static,
    E9: ToString + 'static,
    E10: ToString + 'static,
    E11: ToString + 'static,
    E12: ToString + 'static,
{
    pub fn with_execute<M2A, E2A>(
        self,
        execute_fn: ExecuteFn<M2A, E2A>,
    ) -> ContractBuilder<M1, E1, M2A, M3, M5, M6, E2A, E3, E4, E5, E6, E7, E8, E9, E10, E11, E12>
    where
        M2A: DeserializeOwned + 'static,
        E2A: ToString + 'static,
    {
        ContractBuilder {
            instantiate_fn: self.instantiate_fn,
            execute_fn: Some(execute_fn),
            migrate_fn: self.migrate_fn,
            receive_fn: self.receive_fn,
            reply_fn: self.reply_fn,
            query_fn: self.query_fn,
            before_tx_fn: self.before_tx_fn,
            after_tx_fn: self.after_tx_fn,
            handle_fee_fn: self.handle_fee_fn,
            bank_execute_fn: self.bank_execute_fn,
            bank_query_fn: self.bank_query_fn,
            cron_execute_fn: self.cron_execute_fn,
        }
    }

    pub fn with_migrate<M3A, E3A>(
        self,
        migrate_fn: MigrateFn<M3A, E3A>,
    ) -> ContractBuilder<M1, E1, M2, M3A, M5, M6, E2, E3A, E4, E5, E6, E7, E8, E9, E10, E11, E12>
    where
        M3A: DeserializeOwned + 'static,
        E3A: ToString + 'static,
    {
        ContractBuilder {
            instantiate_fn: self.instantiate_fn,
            execute_fn: self.execute_fn,
            migrate_fn: Some(migrate_fn),
            receive_fn: self.receive_fn,
            reply_fn: self.reply_fn,
            query_fn: self.query_fn,
            before_tx_fn: self.before_tx_fn,
            after_tx_fn: self.after_tx_fn,
            handle_fee_fn: self.handle_fee_fn,
            bank_execute_fn: self.bank_execute_fn,
            bank_query_fn: self.bank_query_fn,
            cron_execute_fn: self.cron_execute_fn,
        }
    }

    pub fn with_receive<E4A>(
        self,
        receive_fn: ReceiveFn<E4A>,
    ) -> ContractBuilder<M1, E1, M2, M3, M5, M6, E2, E3, E4A, E5, E6, E7, E8, E9, E10, E11, E12>
    where
        E4A: ToString + 'static,
    {
        ContractBuilder {
            instantiate_fn: self.instantiate_fn,
            execute_fn: self.execute_fn,
            migrate_fn: self.migrate_fn,
            receive_fn: Some(receive_fn),
            reply_fn: self.reply_fn,
            query_fn: self.query_fn,
            before_tx_fn: self.before_tx_fn,
            after_tx_fn: self.after_tx_fn,
            handle_fee_fn: self.handle_fee_fn,
            bank_execute_fn: self.bank_execute_fn,
            bank_query_fn: self.bank_query_fn,
            cron_execute_fn: self.cron_execute_fn,
        }
    }

    pub fn with_reply<M5A, E5A>(
        self,
        reply_fn: ReplyFn<M5A, E5A>,
    ) -> ContractBuilder<M1, E1, M2, M3, M5A, M6, E2, E3, E4, E5A, E6, E7, E8, E9, E10, E11, E12>
    where
        M5A: DeserializeOwned + 'static,
        E5A: ToString + 'static,
    {
        ContractBuilder {
            instantiate_fn: self.instantiate_fn,
            execute_fn: self.execute_fn,
            migrate_fn: self.migrate_fn,
            receive_fn: self.receive_fn,
            reply_fn: Some(reply_fn),
            query_fn: self.query_fn,
            before_tx_fn: self.before_tx_fn,
            after_tx_fn: self.after_tx_fn,
            handle_fee_fn: self.handle_fee_fn,
            bank_execute_fn: self.bank_execute_fn,
            bank_query_fn: self.bank_query_fn,
            cron_execute_fn: self.cron_execute_fn,
        }
    }

    pub fn with_query<M6A, E6A>(
        self,
        query_fn: QueryFn<M6A, E6A>,
    ) -> ContractBuilder<M1, E1, M2, M3, M5, M6A, E2, E3, E4, E5, E6A, E7, E8, E9, E10, E11, E12>
    where
        M6A: DeserializeOwned + 'static,
        E6A: ToString + 'static,
    {
        ContractBuilder {
            instantiate_fn: self.instantiate_fn,
            execute_fn: self.execute_fn,
            migrate_fn: self.migrate_fn,
            receive_fn: self.receive_fn,
            reply_fn: self.reply_fn,
            query_fn: Some(query_fn),
            before_tx_fn: self.before_tx_fn,
            after_tx_fn: self.after_tx_fn,
            handle_fee_fn: self.handle_fee_fn,
            bank_execute_fn: self.bank_execute_fn,
            bank_query_fn: self.bank_query_fn,
            cron_execute_fn: self.cron_execute_fn,
        }
    }

    pub fn with_before_tx<E7A>(
        self,
        before_tx_fn: BeforeTxFn<E7A>,
    ) -> ContractBuilder<M1, E1, M2, M3, M5, M6, E2, E3, E4, E5, E6, E7A, E8, E9, E10, E11, E12>
    {
        ContractBuilder {
            instantiate_fn: self.instantiate_fn,
            execute_fn: self.execute_fn,
            migrate_fn: self.migrate_fn,
            receive_fn: self.receive_fn,
            reply_fn: self.reply_fn,
            query_fn: self.query_fn,
            before_tx_fn: Some(before_tx_fn),
            after_tx_fn: self.after_tx_fn,
            handle_fee_fn: self.handle_fee_fn,
            bank_execute_fn: self.bank_execute_fn,
            bank_query_fn: self.bank_query_fn,
            cron_execute_fn: self.cron_execute_fn,
        }
    }

    pub fn with_after_tx<E8A>(
        self,
        after_tx_fn: AfterTxFn<E8A>,
    ) -> ContractBuilder<M1, E1, M2, M3, M5, M6, E2, E3, E4, E5, E6, E7, E8A, E9, E10, E11, E12>
    {
        ContractBuilder {
            instantiate_fn: self.instantiate_fn,
            execute_fn: self.execute_fn,
            migrate_fn: self.migrate_fn,
            receive_fn: self.receive_fn,
            reply_fn: self.reply_fn,
            query_fn: self.query_fn,
            before_tx_fn: self.before_tx_fn,
            after_tx_fn: Some(after_tx_fn),
            handle_fee_fn: self.handle_fee_fn,
            bank_execute_fn: self.bank_execute_fn,
            bank_query_fn: self.bank_query_fn,
            cron_execute_fn: self.cron_execute_fn,
        }
    }

    pub fn with_handle_fee<E9A>(
        self,
        hadnle_fee_fn: HandleFeeFn<E9A>,
    ) -> ContractBuilder<M1, E1, M2, M3, M5, M6, E2, E3, E4, E5, E6, E7, E8, E9A, E10, E11, E12>
    {
        ContractBuilder {
            instantiate_fn: self.instantiate_fn,
            execute_fn: self.execute_fn,
            migrate_fn: self.migrate_fn,
            receive_fn: self.receive_fn,
            reply_fn: self.reply_fn,
            query_fn: self.query_fn,
            before_tx_fn: self.before_tx_fn,
            after_tx_fn: self.after_tx_fn,
            handle_fee_fn: Some(hadnle_fee_fn),
            bank_execute_fn: self.bank_execute_fn,
            bank_query_fn: self.bank_query_fn,
            cron_execute_fn: self.cron_execute_fn,
        }
    }

    pub fn with_bank_execute<E10A>(
        self,
        bank_execute_fn: BankExecuteFn<E10A>,
    ) -> ContractBuilder<M1, E1, M2, M3, M5, M6, E2, E3, E4, E5, E6, E7, E8, E9, E10A, E11, E12>
    {
        ContractBuilder {
            instantiate_fn: self.instantiate_fn,
            execute_fn: self.execute_fn,
            migrate_fn: self.migrate_fn,
            receive_fn: self.receive_fn,
            reply_fn: self.reply_fn,
            query_fn: self.query_fn,
            before_tx_fn: self.before_tx_fn,
            after_tx_fn: self.after_tx_fn,
            handle_fee_fn: self.handle_fee_fn,
            bank_execute_fn: Some(bank_execute_fn),
            bank_query_fn: self.bank_query_fn,
            cron_execute_fn: self.cron_execute_fn,
        }
    }

    pub fn with_bank_query<E11A>(
        self,
        bank_query_fn: BankQueryFn<E11A>,
    ) -> ContractBuilder<M1, E1, M2, M3, M5, M6, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11A, E12>
    {
        ContractBuilder {
            instantiate_fn: self.instantiate_fn,
            execute_fn: self.execute_fn,
            migrate_fn: self.migrate_fn,
            receive_fn: self.receive_fn,
            reply_fn: self.reply_fn,
            query_fn: self.query_fn,
            before_tx_fn: self.before_tx_fn,
            after_tx_fn: self.after_tx_fn,
            handle_fee_fn: self.handle_fee_fn,
            bank_execute_fn: self.bank_execute_fn,
            bank_query_fn: Some(bank_query_fn),
            cron_execute_fn: self.cron_execute_fn,
        }
    }

    pub fn with_cron_execute<E12A>(
        self,
        cron_execute_fn: CronExecuteFn<E12A>,
    ) -> ContractBuilder<M1, E1, M2, M3, M5, M6, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11, E12A>
    {
        ContractBuilder {
            instantiate_fn: self.instantiate_fn,
            execute_fn: self.execute_fn,
            migrate_fn: self.migrate_fn,
            receive_fn: self.receive_fn,
            reply_fn: self.reply_fn,
            query_fn: self.query_fn,
            before_tx_fn: self.before_tx_fn,
            after_tx_fn: self.after_tx_fn,
            handle_fee_fn: self.handle_fee_fn,
            bank_execute_fn: self.bank_execute_fn,
            bank_query_fn: self.bank_query_fn,
            cron_execute_fn: Some(cron_execute_fn),
        }
    }

    pub fn build(self) -> ContractWrapper {
        let contracts = CONTRACTS.get_or_init(Default::default);
        let index = contracts.len();

        contracts.push(Box::new(ContractImpl {
            instantiate_fn: self.instantiate_fn,
            execute_fn: self.execute_fn,
            migrate_fn: self.migrate_fn,
            receive_fn: self.receive_fn,
            reply_fn: self.reply_fn,
            query_fn: self.query_fn,
            before_tx_fn: self.before_tx_fn,
            after_tx_fn: self.after_tx_fn,
            handle_fee_fn: self.handle_fee_fn,
            bank_execute_fn: self.bank_execute_fn,
            bank_query_fn: self.bank_query_fn,
            cron_execute_fn: self.cron_execute_fn,
        }));

        ContractWrapper { index }
    }
}

// ----------------------------------- impl ------------------------------------

struct ContractImpl<M1, M2, M3, M5, M6, E1, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11, E12> {
    instantiate_fn: InstantiateFn<M1, E1>,
    execute_fn: Option<ExecuteFn<M2, E2>>,
    migrate_fn: Option<MigrateFn<M3, E3>>,
    receive_fn: Option<ReceiveFn<E4>>,
    reply_fn: Option<ReplyFn<M5, E5>>,
    query_fn: Option<QueryFn<M6, E6>>,
    before_tx_fn: Option<BeforeTxFn<E7>>,
    after_tx_fn: Option<AfterTxFn<E8>>,
    handle_fee_fn: Option<HandleFeeFn<E9>>,
    bank_execute_fn: Option<BankExecuteFn<E10>>,
    bank_query_fn: Option<BankQueryFn<E11>>,
    cron_execute_fn: Option<CronExecuteFn<E12>>,
}

impl<M1, M2, M3, M5, M6, E1, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11, E12> Contract
    for ContractImpl<M1, M2, M3, M5, M6, E1, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11, E12>
where
    M1: DeserializeOwned,
    M2: DeserializeOwned,
    M3: DeserializeOwned,
    M5: DeserializeOwned,
    M6: DeserializeOwned,
    E1: ToString,
    E2: ToString,
    E3: ToString,
    E4: ToString,
    E5: ToString,
    E6: ToString,
    E7: ToString,
    E8: ToString,
    E9: ToString,
    E10: ToString,
    E11: ToString,
    E12: ToString,
{
    fn instantiate(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        msg: &[u8],
    ) -> VmResult<GenericResult<Response>> {
        let mutable_ctx = make_mutable_ctx!(ctx, storage, api, querier);
        let msg = from_json_slice(msg)?;
        let res = (self.instantiate_fn)(mutable_ctx, msg);

        Ok(res.into())
    }

    fn execute(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        msg: &[u8],
    ) -> VmResult<GenericResult<Response>> {
        let Some(execute_fn) = &self.execute_fn else {
            return Err(VmError::function_not_found("execute"));
        };

        let mutable_ctx = make_mutable_ctx!(ctx, storage, api, querier);
        let msg = from_json_slice(msg)?;
        let res = execute_fn(mutable_ctx, msg);

        Ok(res.into())
    }

    fn migrate(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        msg: &[u8],
    ) -> VmResult<GenericResult<Response>> {
        let Some(migrate_fn) = &self.migrate_fn else {
            return Err(VmError::function_not_found("migrate"));
        };

        let mutable_ctx = make_mutable_ctx!(ctx, storage, api, querier);
        let msg = from_json_slice(msg)?;
        let res = migrate_fn(mutable_ctx, msg);

        Ok(res.into())
    }

    fn receive(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
    ) -> VmResult<GenericResult<Response>> {
        let Some(receive_fn) = &self.receive_fn else {
            return Err(VmError::function_not_found("receive"));
        };

        let mutable_ctx = make_mutable_ctx!(ctx, storage, api, querier);
        let res = receive_fn(mutable_ctx);

        Ok(res.into())
    }

    fn reply(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        msg: &[u8],
        submsg_res: SubMsgResult,
    ) -> VmResult<GenericResult<Response>> {
        let Some(reply_fn) = &self.reply_fn else {
            return Err(VmError::function_not_found("reply"));
        };

        let sudo_ctx = make_sudo_ctx!(ctx, storage, api, querier);
        let msg = from_json_slice(msg)?;
        let res = reply_fn(sudo_ctx, msg, submsg_res);

        Ok(res.into())
    }

    fn query(
        &self,
        ctx: Context,
        storage: &dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        msg: &[u8],
    ) -> VmResult<GenericResult<Json>> {
        let Some(query_fn) = &self.query_fn else {
            return Err(VmError::function_not_found("query"));
        };

        let immutable_ctx = make_immutable_ctx!(ctx, storage, api, querier);
        let msg = from_json_slice(msg)?;
        let res = query_fn(immutable_ctx, msg);

        Ok(res.into())
    }

    fn before_tx(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        tx: Tx,
    ) -> VmResult<GenericResult<Response>> {
        let Some(before_tx_fn) = &self.before_tx_fn else {
            return Err(VmError::function_not_found("before_tx"));
        };

        let auth_ctx = make_auth_ctx!(ctx, storage, api, querier);
        let res = before_tx_fn(auth_ctx, tx);

        Ok(res.into())
    }

    fn after_tx(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        tx: Tx,
    ) -> VmResult<GenericResult<Response>> {
        let Some(after_tx_fn) = &self.after_tx_fn else {
            return Err(VmError::function_not_found("after_tx"));
        };

        let auth_ctx = make_auth_ctx!(ctx, storage, api, querier);
        let res = after_tx_fn(auth_ctx, tx);

        Ok(res.into())
    }

    fn handle_fee(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        tx: Tx,
        outcome: Outcome,
    ) -> VmResult<GenericResult<Response>> {
        let Some(handle_fee_fn) = &self.handle_fee_fn else {
            return Err(VmError::function_not_found("handle_fee"));
        };

        let sudo_ctx = make_sudo_ctx!(ctx, storage, api, querier);
        let res = handle_fee_fn(sudo_ctx, tx, outcome);

        Ok(res.into())
    }

    fn bank_execute(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        msg: BankMsg,
    ) -> VmResult<GenericResult<Response>> {
        let Some(bank_execute_fn) = &self.bank_execute_fn else {
            return Err(VmError::function_not_found("bank_execute"));
        };

        let sudo_ctx = make_sudo_ctx!(ctx, storage, api, querier);
        let res = bank_execute_fn(sudo_ctx, msg);

        Ok(res.into())
    }

    fn bank_query(
        &self,
        ctx: Context,
        storage: &dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        msg: BankQuery,
    ) -> VmResult<GenericResult<BankQueryResponse>> {
        let Some(bank_query_fn) = &self.bank_query_fn else {
            return Err(VmError::function_not_found("bank_query"));
        };

        let immutable_ctx = make_immutable_ctx!(ctx, storage, api, querier);
        let res = bank_query_fn(immutable_ctx, msg);

        Ok(res.into())
    }

    fn cron_execute(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
    ) -> VmResult<GenericResult<Response>> {
        let Some(cron_execute_fn) = &self.cron_execute_fn else {
            return Err(VmError::function_not_found("cron_execute"));
        };

        let sudo_ctx = make_sudo_ctx!(ctx, storage, api, querier);
        let res = cron_execute_fn(sudo_ctx);

        Ok(res.into())
    }
}
