use {
    crate::{
        AuthenticateFn, BackrunFn, BankExecuteFn, BankQueryFn, Contract, CronExecuteFn, ExecuteFn,
        FinalizeFeeFn, InstantiateFn, MigrateFn, QueryFn, ReceiveFn, ReplyFn, VmError, VmResult,
        WithholdFeeFn,
    },
    elsa::sync::FrozenVec,
    grug_types::{
        Api, AuthCtx, AuthResponse, BankMsg, BankQuery, BankQueryResponse, Binary, BorshDeExt,
        Context, Empty, GenericResult, GenericResultExt, ImmutableCtx, Json, JsonDeExt, MutableCtx,
        Querier, QuerierWrapper, Response, StdError, Storage, SubMsgResult, SudoCtx, Tx, TxOutcome,
        make_auth_ctx, make_immutable_ctx, make_mutable_ctx, make_sudo_ctx,
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

impl ContractWrapper {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            index: usize::from_le_bytes(bytes.try_into().unwrap()),
        }
    }

    pub fn to_bytes(&self) -> [u8; usize::BITS as usize / 8] {
        self.index.to_le_bytes()
    }
}

impl From<ContractWrapper> for Binary {
    fn from(wrapper: ContractWrapper) -> Self {
        wrapper.to_bytes().into()
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
    E13 = StdError,
> {
    instantiate_fn: InstantiateFn<M1, E1>,
    execute_fn: Option<ExecuteFn<M2, E2>>,
    migrate_fn: Option<MigrateFn<M3, E3>>,
    receive_fn: Option<ReceiveFn<E4>>,
    reply_fn: Option<ReplyFn<M5, E5>>,
    query_fn: Option<QueryFn<M6, E6>>,
    authenticate_fn: Option<AuthenticateFn<E7>>,
    backrun_fn: Option<BackrunFn<E8>>,
    bank_execute_fn: Option<BankExecuteFn<E9>>,
    bank_query_fn: Option<BankQueryFn<E10>>,
    withhold_fee_fn: Option<WithholdFeeFn<E11>>,
    finalize_fee_fn: Option<FinalizeFeeFn<E12>>,
    cron_execute_fn: Option<CronExecuteFn<E13>>,
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
            authenticate_fn: None,
            backrun_fn: None,
            bank_execute_fn: None,
            bank_query_fn: None,
            withhold_fee_fn: None,
            finalize_fee_fn: None,
            cron_execute_fn: None,
        }
    }
}

impl<M1, E1, M2, M3, M5, M6, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11, E12, E13>
    ContractBuilder<M1, E1, M2, M3, M5, M6, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11, E12, E13>
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
    E13: ToString + 'static,
{
    pub fn with_execute<M2A, E2A>(
        self,
        execute_fn: ExecuteFn<M2A, E2A>,
    ) -> ContractBuilder<M1, E1, M2A, M3, M5, M6, E2A, E3, E4, E5, E6, E7, E8, E9, E10, E11, E12, E13>
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
            authenticate_fn: self.authenticate_fn,
            backrun_fn: self.backrun_fn,
            bank_execute_fn: self.bank_execute_fn,
            bank_query_fn: self.bank_query_fn,
            withhold_fee_fn: self.withhold_fee_fn,
            finalize_fee_fn: self.finalize_fee_fn,
            cron_execute_fn: self.cron_execute_fn,
        }
    }

    pub fn with_migrate<M3A, E3A>(
        self,
        migrate_fn: MigrateFn<M3A, E3A>,
    ) -> ContractBuilder<M1, E1, M2, M3A, M5, M6, E2, E3A, E4, E5, E6, E7, E8, E9, E10, E11, E12, E13>
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
            authenticate_fn: self.authenticate_fn,
            backrun_fn: self.backrun_fn,
            bank_execute_fn: self.bank_execute_fn,
            bank_query_fn: self.bank_query_fn,
            withhold_fee_fn: self.withhold_fee_fn,
            finalize_fee_fn: self.finalize_fee_fn,
            cron_execute_fn: self.cron_execute_fn,
        }
    }

    pub fn with_receive<E4A>(
        self,
        receive_fn: ReceiveFn<E4A>,
    ) -> ContractBuilder<M1, E1, M2, M3, M5, M6, E2, E3, E4A, E5, E6, E7, E8, E9, E10, E11, E12, E13>
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
            authenticate_fn: self.authenticate_fn,
            backrun_fn: self.backrun_fn,
            bank_execute_fn: self.bank_execute_fn,
            bank_query_fn: self.bank_query_fn,
            withhold_fee_fn: self.withhold_fee_fn,
            finalize_fee_fn: self.finalize_fee_fn,
            cron_execute_fn: self.cron_execute_fn,
        }
    }

    pub fn with_reply<M5A, E5A>(
        self,
        reply_fn: ReplyFn<M5A, E5A>,
    ) -> ContractBuilder<M1, E1, M2, M3, M5A, M6, E2, E3, E4, E5A, E6, E7, E8, E9, E10, E11, E12, E13>
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
            authenticate_fn: self.authenticate_fn,
            backrun_fn: self.backrun_fn,
            bank_execute_fn: self.bank_execute_fn,
            bank_query_fn: self.bank_query_fn,
            withhold_fee_fn: self.withhold_fee_fn,
            finalize_fee_fn: self.finalize_fee_fn,
            cron_execute_fn: self.cron_execute_fn,
        }
    }

    pub fn with_query<M6A, E6A>(
        self,
        query_fn: QueryFn<M6A, E6A>,
    ) -> ContractBuilder<M1, E1, M2, M3, M5, M6A, E2, E3, E4, E5, E6A, E7, E8, E9, E10, E11, E12, E13>
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
            authenticate_fn: self.authenticate_fn,
            backrun_fn: self.backrun_fn,
            bank_execute_fn: self.bank_execute_fn,
            bank_query_fn: self.bank_query_fn,
            withhold_fee_fn: self.withhold_fee_fn,
            finalize_fee_fn: self.finalize_fee_fn,
            cron_execute_fn: self.cron_execute_fn,
        }
    }

    pub fn with_authenticate<E7A>(
        self,
        authenticate_fn: AuthenticateFn<E7A>,
    ) -> ContractBuilder<M1, E1, M2, M3, M5, M6, E2, E3, E4, E5, E6, E7A, E8, E9, E10, E11, E12, E13>
    {
        ContractBuilder {
            instantiate_fn: self.instantiate_fn,
            execute_fn: self.execute_fn,
            migrate_fn: self.migrate_fn,
            receive_fn: self.receive_fn,
            reply_fn: self.reply_fn,
            query_fn: self.query_fn,
            authenticate_fn: Some(authenticate_fn),
            backrun_fn: self.backrun_fn,
            bank_execute_fn: self.bank_execute_fn,
            bank_query_fn: self.bank_query_fn,
            withhold_fee_fn: self.withhold_fee_fn,
            finalize_fee_fn: self.finalize_fee_fn,
            cron_execute_fn: self.cron_execute_fn,
        }
    }

    pub fn with_backrun<E8A>(
        self,
        backrun_fn: BackrunFn<E8A>,
    ) -> ContractBuilder<M1, E1, M2, M3, M5, M6, E2, E3, E4, E5, E6, E7, E8A, E9, E10, E11, E12, E13>
    {
        ContractBuilder {
            instantiate_fn: self.instantiate_fn,
            execute_fn: self.execute_fn,
            migrate_fn: self.migrate_fn,
            receive_fn: self.receive_fn,
            reply_fn: self.reply_fn,
            query_fn: self.query_fn,
            authenticate_fn: self.authenticate_fn,
            backrun_fn: Some(backrun_fn),
            bank_execute_fn: self.bank_execute_fn,
            bank_query_fn: self.bank_query_fn,
            withhold_fee_fn: self.withhold_fee_fn,
            finalize_fee_fn: self.finalize_fee_fn,
            cron_execute_fn: self.cron_execute_fn,
        }
    }

    pub fn with_bank_execute<E9A>(
        self,
        bank_execute_fn: BankExecuteFn<E9A>,
    ) -> ContractBuilder<M1, E1, M2, M3, M5, M6, E2, E3, E4, E5, E6, E7, E8, E9A, E10, E11, E12, E13>
    {
        ContractBuilder {
            instantiate_fn: self.instantiate_fn,
            execute_fn: self.execute_fn,
            migrate_fn: self.migrate_fn,
            receive_fn: self.receive_fn,
            reply_fn: self.reply_fn,
            query_fn: self.query_fn,
            authenticate_fn: self.authenticate_fn,
            backrun_fn: self.backrun_fn,
            bank_execute_fn: Some(bank_execute_fn),
            bank_query_fn: self.bank_query_fn,
            withhold_fee_fn: self.withhold_fee_fn,
            finalize_fee_fn: self.finalize_fee_fn,
            cron_execute_fn: self.cron_execute_fn,
        }
    }

    pub fn with_bank_query<E10A>(
        self,
        bank_query_fn: BankQueryFn<E10A>,
    ) -> ContractBuilder<M1, E1, M2, M3, M5, M6, E2, E3, E4, E5, E6, E7, E8, E9, E10A, E11, E12, E13>
    {
        ContractBuilder {
            instantiate_fn: self.instantiate_fn,
            execute_fn: self.execute_fn,
            migrate_fn: self.migrate_fn,
            receive_fn: self.receive_fn,
            reply_fn: self.reply_fn,
            query_fn: self.query_fn,
            authenticate_fn: self.authenticate_fn,
            backrun_fn: self.backrun_fn,
            bank_execute_fn: self.bank_execute_fn,
            bank_query_fn: Some(bank_query_fn),
            withhold_fee_fn: self.withhold_fee_fn,
            finalize_fee_fn: self.finalize_fee_fn,
            cron_execute_fn: self.cron_execute_fn,
        }
    }

    pub fn with_withhold_fee<E11A>(
        self,
        withhold_fee_fn: WithholdFeeFn<E11A>,
    ) -> ContractBuilder<M1, E1, M2, M3, M5, M6, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11A, E12, E13>
    {
        ContractBuilder {
            instantiate_fn: self.instantiate_fn,
            execute_fn: self.execute_fn,
            migrate_fn: self.migrate_fn,
            receive_fn: self.receive_fn,
            reply_fn: self.reply_fn,
            query_fn: self.query_fn,
            authenticate_fn: self.authenticate_fn,
            backrun_fn: self.backrun_fn,
            bank_execute_fn: self.bank_execute_fn,
            bank_query_fn: self.bank_query_fn,
            withhold_fee_fn: Some(withhold_fee_fn),
            finalize_fee_fn: self.finalize_fee_fn,
            cron_execute_fn: self.cron_execute_fn,
        }
    }

    pub fn with_finalize_fee<E12A>(
        self,
        finalize_fee_fn: FinalizeFeeFn<E12A>,
    ) -> ContractBuilder<M1, E1, M2, M3, M5, M6, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11, E12A, E13>
    {
        ContractBuilder {
            instantiate_fn: self.instantiate_fn,
            execute_fn: self.execute_fn,
            migrate_fn: self.migrate_fn,
            receive_fn: self.receive_fn,
            reply_fn: self.reply_fn,
            query_fn: self.query_fn,
            authenticate_fn: self.authenticate_fn,
            backrun_fn: self.backrun_fn,
            bank_execute_fn: self.bank_execute_fn,
            bank_query_fn: self.bank_query_fn,
            withhold_fee_fn: self.withhold_fee_fn,
            finalize_fee_fn: Some(finalize_fee_fn),
            cron_execute_fn: self.cron_execute_fn,
        }
    }

    pub fn with_cron_execute<E13A>(
        self,
        cron_execute_fn: CronExecuteFn<E13A>,
    ) -> ContractBuilder<M1, E1, M2, M3, M5, M6, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11, E12, E13A>
    {
        ContractBuilder {
            instantiate_fn: self.instantiate_fn,
            execute_fn: self.execute_fn,
            migrate_fn: self.migrate_fn,
            receive_fn: self.receive_fn,
            reply_fn: self.reply_fn,
            query_fn: self.query_fn,
            authenticate_fn: self.authenticate_fn,
            backrun_fn: self.backrun_fn,
            bank_execute_fn: self.bank_execute_fn,
            bank_query_fn: self.bank_query_fn,
            withhold_fee_fn: self.withhold_fee_fn,
            finalize_fee_fn: self.finalize_fee_fn,
            cron_execute_fn: Some(cron_execute_fn),
        }
    }

    pub fn build(self) -> ContractWrapper {
        let index = CONTRACTS
            .get_or_init(Default::default)
            .push_get_index(Box::new(ContractImpl {
                instantiate_fn: self.instantiate_fn,
                execute_fn: self.execute_fn,
                migrate_fn: self.migrate_fn,
                receive_fn: self.receive_fn,
                reply_fn: self.reply_fn,
                query_fn: self.query_fn,
                authenticate_fn: self.authenticate_fn,
                backrun_fn: self.backrun_fn,
                bank_execute_fn: self.bank_execute_fn,
                bank_query_fn: self.bank_query_fn,
                withhold_fee_fn: self.withhold_fee_fn,
                finalize_fee_fn: self.finalize_fee_fn,
                cron_execute_fn: self.cron_execute_fn,
            }));

        ContractWrapper { index }
    }
}

// ----------------------------------- impl ------------------------------------

struct ContractImpl<M1, M2, M3, M5, M6, E1, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11, E12, E13> {
    instantiate_fn: InstantiateFn<M1, E1>,
    execute_fn: Option<ExecuteFn<M2, E2>>,
    migrate_fn: Option<MigrateFn<M3, E3>>,
    receive_fn: Option<ReceiveFn<E4>>,
    reply_fn: Option<ReplyFn<M5, E5>>,
    query_fn: Option<QueryFn<M6, E6>>,
    authenticate_fn: Option<AuthenticateFn<E7>>,
    backrun_fn: Option<BackrunFn<E8>>,
    bank_execute_fn: Option<BankExecuteFn<E9>>,
    bank_query_fn: Option<BankQueryFn<E10>>,
    withhold_fee_fn: Option<WithholdFeeFn<E11>>,
    finalize_fee_fn: Option<FinalizeFeeFn<E12>>,
    cron_execute_fn: Option<CronExecuteFn<E13>>,
}

impl<M1, M2, M3, M5, M6, E1, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11, E12, E13> Contract
    for ContractImpl<M1, M2, M3, M5, M6, E1, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11, E12, E13>
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
    E13: ToString,
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
        let msg = msg.deserialize_borsh::<Json>()?.deserialize_json()?;
        let res = (self.instantiate_fn)(mutable_ctx, msg);

        Ok(res.into_generic_result())
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
        let msg = msg.deserialize_borsh::<Json>()?.deserialize_json()?;
        let res = execute_fn(mutable_ctx, msg);

        Ok(res.into_generic_result())
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

        let sudo_ctx = make_sudo_ctx!(ctx, storage, api, querier);
        let msg = msg.deserialize_borsh::<Json>()?.deserialize_json()?;
        let res = migrate_fn(sudo_ctx, msg);

        Ok(res.into_generic_result())
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

        Ok(res.into_generic_result())
    }

    fn reply(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        msg: &[u8],
        result: SubMsgResult,
    ) -> VmResult<GenericResult<Response>> {
        let Some(reply_fn) = &self.reply_fn else {
            return Err(VmError::function_not_found("reply"));
        };

        let sudo_ctx = make_sudo_ctx!(ctx, storage, api, querier);
        let msg = msg.deserialize_borsh::<Json>()?.deserialize_json()?;
        let res = reply_fn(sudo_ctx, msg, result);

        Ok(res.into_generic_result())
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
        let msg = msg.deserialize_borsh::<Json>()?.deserialize_json()?;
        let res = query_fn(immutable_ctx, msg);

        Ok(res.into_generic_result())
    }

    fn authenticate(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        tx: Tx,
    ) -> VmResult<GenericResult<AuthResponse>> {
        let Some(authenticate_fn) = &self.authenticate_fn else {
            return Err(VmError::function_not_found("authenticate"));
        };

        let auth_ctx = make_auth_ctx!(ctx, storage, api, querier);
        let res = authenticate_fn(auth_ctx, tx);

        Ok(res.into_generic_result())
    }

    fn backrun(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        tx: Tx,
    ) -> VmResult<GenericResult<Response>> {
        let Some(backrun_fn) = &self.backrun_fn else {
            return Err(VmError::function_not_found("backrun"));
        };

        let auth_ctx = make_auth_ctx!(ctx, storage, api, querier);
        let res = backrun_fn(auth_ctx, tx);

        Ok(res.into_generic_result())
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

        Ok(res.into_generic_result())
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

        Ok(res.into_generic_result())
    }

    fn withhold_fee(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        tx: Tx,
    ) -> VmResult<GenericResult<Response>> {
        let Some(withhold_fee_fn) = &self.withhold_fee_fn else {
            return Err(VmError::function_not_found("withhold_fee"));
        };

        let auth_ctx = make_auth_ctx!(ctx, storage, api, querier);
        let res = withhold_fee_fn(auth_ctx, tx);

        Ok(res.into_generic_result())
    }

    fn finalize_fee(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        tx: Tx,
        outcome: TxOutcome,
    ) -> VmResult<GenericResult<Response>> {
        let Some(finalize_fee_fn) = &self.finalize_fee_fn else {
            return Err(VmError::function_not_found("finalize_fee"));
        };

        let auth_ctx = make_auth_ctx!(ctx, storage, api, querier);
        let res = finalize_fee_fn(auth_ctx, tx, outcome);

        Ok(res.into_generic_result())
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

        Ok(res.into_generic_result())
    }
}
