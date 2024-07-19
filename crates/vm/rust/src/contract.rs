use {
    crate::{
        AfterTxFn, BankExecuteFn, BankQueryFn, BeforeTxFn, Contract, CronExecuteFn, ExecuteFn,
        InstantiateFn, MigrateFn, QueryFn, ReceiveFn, ReplyFn,
    },
    elsa::sync::FrozenVec,
    grug_types::{
        from_json_value, make_auth_ctx, make_immutable_ctx, make_mutable_ctx, make_sudo_ctx,
        return_into_generic_result, unwrap_into_generic_result, Api, AuthCtx, BankMsg, BankQuery,
        BankQueryResponse, Binary, Context, Empty, GenericResult, ImmutableCtx, Json, MutableCtx,
        Querier, QuerierWrapper, Response, StdError, Storage, SubMsgResult, SudoCtx, Tx,
    },
    serde::de::DeserializeOwned,
    std::sync::OnceLock,
};

pub(crate) static CONTRACTS: OnceLock<FrozenVec<Box<dyn Contract + Send + Sync>>> = OnceLock::new();

// ---------------------------------- wrapper ----------------------------------

#[derive(Clone)]
pub struct ContractWrapper {
    pub(crate) index: usize,
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
> {
    instantiate_fn: InstantiateFn<M1, E1>,
    execute_fn: Option<ExecuteFn<M2, E2>>,
    migrate_fn: Option<MigrateFn<M3, E3>>,
    receive_fn: Option<ReceiveFn<E4>>,
    reply_fn: Option<ReplyFn<M5, E5>>,
    query_fn: Option<QueryFn<M6, E6>>,
    before_tx_fn: Option<BeforeTxFn<E7>>,
    after_tx_fn: Option<AfterTxFn<E8>>,
    bank_execute_fn: Option<BankExecuteFn<E9>>,
    bank_query_fn: Option<BankQueryFn<E10>>,
    cron_execute_fn: Option<CronExecuteFn<E11>>,
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
            bank_execute_fn: None,
            bank_query_fn: None,
            cron_execute_fn: None,
        }
    }
}

impl<M1, E1, M2, M3, M5, M6, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11>
    ContractBuilder<M1, E1, M2, M3, M5, M6, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11>
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
{
    pub fn with_execute<M2A, E2A>(
        self,
        execute_fn: ExecuteFn<M2A, E2A>,
    ) -> ContractBuilder<M1, E1, M2A, M3, M5, M6, E2A, E3, E4, E5, E6, E7, E8, E9, E10, E11>
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
            bank_execute_fn: self.bank_execute_fn,
            bank_query_fn: self.bank_query_fn,
            cron_execute_fn: self.cron_execute_fn,
        }
    }

    pub fn with_migrate<M3A, E3A>(
        self,
        migrate_fn: MigrateFn<M3A, E3A>,
    ) -> ContractBuilder<M1, E1, M2, M3A, M5, M6, E2, E3A, E4, E5, E6, E7, E8, E9, E10, E11>
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
            bank_execute_fn: self.bank_execute_fn,
            bank_query_fn: self.bank_query_fn,
            cron_execute_fn: self.cron_execute_fn,
        }
    }

    pub fn with_receive<E4A>(
        self,
        receive_fn: ReceiveFn<E4A>,
    ) -> ContractBuilder<M1, E1, M2, M3, M5, M6, E2, E3, E4A, E5, E6, E7, E8, E9, E10, E11>
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
            bank_execute_fn: self.bank_execute_fn,
            bank_query_fn: self.bank_query_fn,
            cron_execute_fn: self.cron_execute_fn,
        }
    }

    pub fn with_reply<M5A, E5A>(
        self,
        reply_fn: ReplyFn<M5A, E5A>,
    ) -> ContractBuilder<M1, E1, M2, M3, M5A, M6, E2, E3, E4, E5A, E6, E7, E8, E9, E10, E11>
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
            bank_execute_fn: self.bank_execute_fn,
            bank_query_fn: self.bank_query_fn,
            cron_execute_fn: self.cron_execute_fn,
        }
    }

    pub fn with_query<M6A, E6A>(
        self,
        query_fn: QueryFn<M6A, E6A>,
    ) -> ContractBuilder<M1, E1, M2, M3, M5, M6A, E2, E3, E4, E5, E6A, E7, E8, E9, E10, E11>
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
            bank_execute_fn: self.bank_execute_fn,
            bank_query_fn: self.bank_query_fn,
            cron_execute_fn: self.cron_execute_fn,
        }
    }

    pub fn with_before_tx<E7A>(
        self,
        before_tx_fn: BeforeTxFn<E7A>,
    ) -> ContractBuilder<M1, E1, M2, M3, M5, M6, E2, E3, E4, E5, E6, E7A, E8, E9, E10, E11> {
        ContractBuilder {
            instantiate_fn: self.instantiate_fn,
            execute_fn: self.execute_fn,
            migrate_fn: self.migrate_fn,
            receive_fn: self.receive_fn,
            reply_fn: self.reply_fn,
            query_fn: self.query_fn,
            before_tx_fn: Some(before_tx_fn),
            after_tx_fn: self.after_tx_fn,
            bank_execute_fn: self.bank_execute_fn,
            bank_query_fn: self.bank_query_fn,
            cron_execute_fn: self.cron_execute_fn,
        }
    }

    pub fn with_after_tx<E8A>(
        self,
        after_tx_fn: AfterTxFn<E8A>,
    ) -> ContractBuilder<M1, E1, M2, M3, M5, M6, E2, E3, E4, E5, E6, E7, E8A, E9, E10, E11> {
        ContractBuilder {
            instantiate_fn: self.instantiate_fn,
            execute_fn: self.execute_fn,
            migrate_fn: self.migrate_fn,
            receive_fn: self.receive_fn,
            reply_fn: self.reply_fn,
            query_fn: self.query_fn,
            before_tx_fn: self.before_tx_fn,
            after_tx_fn: Some(after_tx_fn),
            bank_execute_fn: self.bank_execute_fn,
            bank_query_fn: self.bank_query_fn,
            cron_execute_fn: self.cron_execute_fn,
        }
    }

    pub fn with_bank_execute<E9A>(
        self,
        bank_execute_fn: BankExecuteFn<E9A>,
    ) -> ContractBuilder<M1, E1, M2, M3, M5, M6, E2, E3, E4, E5, E6, E7, E8, E9A, E10, E11> {
        ContractBuilder {
            instantiate_fn: self.instantiate_fn,
            execute_fn: self.execute_fn,
            migrate_fn: self.migrate_fn,
            receive_fn: self.receive_fn,
            reply_fn: self.reply_fn,
            query_fn: self.query_fn,
            before_tx_fn: self.before_tx_fn,
            after_tx_fn: self.after_tx_fn,
            bank_execute_fn: Some(bank_execute_fn),
            bank_query_fn: self.bank_query_fn,
            cron_execute_fn: self.cron_execute_fn,
        }
    }

    pub fn with_bank_query<E10A>(
        self,
        bank_query_fn: BankQueryFn<E10A>,
    ) -> ContractBuilder<M1, E1, M2, M3, M5, M6, E2, E3, E4, E5, E6, E7, E8, E9, E10A, E11> {
        ContractBuilder {
            instantiate_fn: self.instantiate_fn,
            execute_fn: self.execute_fn,
            migrate_fn: self.migrate_fn,
            receive_fn: self.receive_fn,
            reply_fn: self.reply_fn,
            query_fn: self.query_fn,
            before_tx_fn: self.before_tx_fn,
            after_tx_fn: self.after_tx_fn,
            bank_execute_fn: self.bank_execute_fn,
            bank_query_fn: Some(bank_query_fn),
            cron_execute_fn: self.cron_execute_fn,
        }
    }

    pub fn with_cron_execute<E11A>(
        self,
        cron_execute_fn: CronExecuteFn<E11A>,
    ) -> ContractBuilder<M1, E1, M2, M3, M5, M6, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11A> {
        ContractBuilder {
            instantiate_fn: self.instantiate_fn,
            execute_fn: self.execute_fn,
            migrate_fn: self.migrate_fn,
            receive_fn: self.receive_fn,
            reply_fn: self.reply_fn,
            query_fn: self.query_fn,
            before_tx_fn: self.before_tx_fn,
            after_tx_fn: self.after_tx_fn,
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
            bank_execute_fn: self.bank_execute_fn,
            bank_query_fn: self.bank_query_fn,
            cron_execute_fn: self.cron_execute_fn,
        }));

        ContractWrapper { index }
    }
}

// ----------------------------------- impl ------------------------------------

pub struct ContractImpl<M1, M2, M3, M5, M6, E1, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11> {
    instantiate_fn: InstantiateFn<M1, E1>,
    execute_fn: Option<ExecuteFn<M2, E2>>,
    migrate_fn: Option<MigrateFn<M3, E3>>,
    receive_fn: Option<ReceiveFn<E4>>,
    reply_fn: Option<ReplyFn<M5, E5>>,
    query_fn: Option<QueryFn<M6, E6>>,
    before_tx_fn: Option<BeforeTxFn<E7>>,
    after_tx_fn: Option<AfterTxFn<E8>>,
    bank_execute_fn: Option<BankExecuteFn<E9>>,
    bank_query_fn: Option<BankQueryFn<E10>>,
    cron_execute_fn: Option<CronExecuteFn<E11>>,
}

impl<M1, M2, M3, M5, M6, E1, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11> Contract
    for ContractImpl<M1, M2, M3, M5, M6, E1, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11>
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
{
    fn instantiate(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        msg: Json,
    ) -> GenericResult<Response> {
        let mutable_ctx = make_mutable_ctx!(ctx, storage, api, querier);
        let msg = unwrap_into_generic_result!(from_json_value(msg));
        return_into_generic_result!((self.instantiate_fn)(mutable_ctx, msg))
    }

    fn execute(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        msg: Json,
    ) -> GenericResult<Response> {
        let mutable_ctx = make_mutable_ctx!(ctx, storage, api, querier);
        let msg = unwrap_into_generic_result!(from_json_value(msg));
        // TODO: gracefully handle the `Option` instead of unwrapping??
        return_into_generic_result!(self.execute_fn.as_ref().unwrap()(mutable_ctx, msg))
    }

    fn migrate(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        msg: Json,
    ) -> GenericResult<Response> {
        let mutable_ctx = make_mutable_ctx!(ctx, storage, api, querier);
        let msg = unwrap_into_generic_result!(from_json_value(msg));
        return_into_generic_result!(self.migrate_fn.as_ref().unwrap()(mutable_ctx, msg))
    }

    fn receive(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
    ) -> GenericResult<Response> {
        let mutable_ctx = make_mutable_ctx!(ctx, storage, api, querier);
        return_into_generic_result!(self.receive_fn.as_ref().unwrap()(mutable_ctx))
    }

    fn reply(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        msg: Json,
        submsg_res: SubMsgResult,
    ) -> GenericResult<Response> {
        let sudo_ctx = make_sudo_ctx!(ctx, storage, api, querier);
        let msg = unwrap_into_generic_result!(from_json_value(msg));
        return_into_generic_result!(self.reply_fn.as_ref().unwrap()(sudo_ctx, msg, submsg_res))
    }

    fn query(
        &self,
        ctx: Context,
        storage: &dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        msg: Json,
    ) -> GenericResult<Json> {
        let immutable_ctx = make_immutable_ctx!(ctx, storage, api, querier);
        let msg = unwrap_into_generic_result!(from_json_value(msg));
        return_into_generic_result!(self.query_fn.as_ref().unwrap()(immutable_ctx, msg))
    }

    fn before_tx(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        tx: Tx,
    ) -> GenericResult<Response> {
        let auth_ctx = make_auth_ctx!(ctx, storage, api, querier);
        return_into_generic_result!(self.before_tx_fn.as_ref().unwrap()(auth_ctx, tx))
    }

    fn after_tx(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        tx: Tx,
    ) -> GenericResult<Response> {
        let auth_ctx = make_auth_ctx!(ctx, storage, api, querier);
        return_into_generic_result!(self.after_tx_fn.as_ref().unwrap()(auth_ctx, tx))
    }

    fn bank_execute(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        msg: BankMsg,
    ) -> GenericResult<Response> {
        let sudo_ctx = make_sudo_ctx!(ctx, storage, api, querier);
        return_into_generic_result!(self.bank_execute_fn.as_ref().unwrap()(sudo_ctx, msg))
    }

    fn bank_query(
        &self,
        ctx: Context,
        storage: &dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        msg: BankQuery,
    ) -> GenericResult<BankQueryResponse> {
        let immutable_ctx = make_immutable_ctx!(ctx, storage, api, querier);
        return_into_generic_result!(self.bank_query_fn.as_ref().unwrap()(immutable_ctx, msg))
    }

    fn cron_execute(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
    ) -> GenericResult<Response> {
        let sudo_ctx = make_sudo_ctx!(ctx, storage, api, querier);
        return_into_generic_result!(self.cron_execute_fn.as_ref().unwrap()(sudo_ctx))
    }
}
