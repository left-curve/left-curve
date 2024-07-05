use {
    crate::{Contract, ExecuteFn, InstantiateFn, MigrateFn, QueryFn, ReceiveFn, ReplyFn},
    elsa::sync::FrozenVec,
    grug_types::{
        from_json_value, make_immutable_ctx, make_mutable_ctx, make_sudo_ctx,
        return_into_generic_result, unwrap_into_generic_result, Api, Context, Empty, GenericResult,
        ImmutableCtx, Json, MutableCtx, Querier, QuerierWrapper, Response, StdError, Storage,
        SubMsgResult, SudoCtx,
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

impl ContractWrapper {
    pub fn into_bytes(self) -> Vec<u8> {
        self.index.to_le_bytes().to_vec()
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            index: usize::from_le_bytes(bytes.try_into().unwrap()),
        }
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
> {
    instantiate_fn: InstantiateFn<M1, E1>,
    execute_fn: Option<ExecuteFn<M2, E2>>,
    migrate_fn: Option<MigrateFn<M3, E3>>,
    receive_fn: Option<ReceiveFn<E4>>,
    reply_fn: Option<ReplyFn<M5, E5>>,
    query_fn: Option<QueryFn<M6, E6>>,
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
        }
    }
}

impl<M1, E1, M2, M3, M5, M6, E2, E3, E4, E5, E6>
    ContractBuilder<M1, E1, M2, M3, M5, M6, E2, E3, E4, E5, E6>
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
{
    pub fn with_execute<M2A, E2A>(
        self,
        execute_fn: ExecuteFn<M2A, E2A>,
    ) -> ContractBuilder<M1, E1, M2A, M3, M5, M6, E2A, E3, E4, E5, E6>
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
        }
    }

    pub fn with_migrate<M3A, E3A>(
        self,
        migrate_fn: MigrateFn<M3A, E3A>,
    ) -> ContractBuilder<M1, E1, M2, M3A, M5, M6, E2, E3A, E4, E5, E6>
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
        }
    }

    pub fn with_receive<E4A>(
        self,
        receive_fn: ReceiveFn<E4A>,
    ) -> ContractBuilder<M1, E1, M2, M3, M5, M6, E2, E3, E4A, E5, E6>
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
        }
    }

    pub fn with_reply<M5A, E5A>(
        self,
        reply_fn: ReplyFn<M5A, E5A>,
    ) -> ContractBuilder<M1, E1, M2, M3, M5A, M6, E2, E3, E4, E5A, E6>
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
        }
    }

    pub fn with_query<M6A, E6A>(
        self,
        query_fn: QueryFn<M6A, E6A>,
    ) -> ContractBuilder<M1, E1, M2, M3, M5, M6A, E2, E3, E4, E5, E6A>
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
        }));
        ContractWrapper { index }
    }
}

// ----------------------------------- impl ------------------------------------

pub struct ContractImpl<M1, M2, M3, M5, M6, E1, E2, E3, E4, E5, E6> {
    instantiate_fn: InstantiateFn<M1, E1>,
    execute_fn: Option<ExecuteFn<M2, E2>>,
    migrate_fn: Option<MigrateFn<M3, E3>>,
    receive_fn: Option<ReceiveFn<E4>>,
    reply_fn: Option<ReplyFn<M5, E5>>,
    query_fn: Option<QueryFn<M6, E6>>,
}

impl<M1, M2, M3, M5, M6, E1, E2, E3, E4, E5, E6> Contract
    for ContractImpl<M1, M2, M3, M5, M6, E1, E2, E3, E4, E5, E6>
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
}
