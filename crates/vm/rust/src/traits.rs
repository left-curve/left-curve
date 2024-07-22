// Skip formatting this entire file
// https://stackoverflow.com/questions/59247458/is-there-a-stable-way-to-tell-rustfmt-to-skip-an-entire-file
#![cfg_attr(rustfmt, rustfmt::skip)]

use {
    crate::VmResult,
    grug_types::{
        Api, AuthCtx, BankMsg, BankQuery, BankQueryResponse, Context, GenericResult, ImmutableCtx, Json, MutableCtx, Outcome, Querier, Response, Storage, SubMsgResult, SudoCtx, Tx
    },
};

pub trait Contract {
    fn instantiate(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        msg: &[u8],
    ) -> VmResult<GenericResult<Response>>;

    fn execute(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        msg: &[u8],
    ) -> VmResult<GenericResult<Response>>;

    fn migrate(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        msg: &[u8],
    ) -> VmResult<GenericResult<Response>>;

    fn receive(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
    ) -> VmResult<GenericResult<Response>>;

    fn reply(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        msg: &[u8],
        submsg_res: SubMsgResult,
    ) -> VmResult<GenericResult<Response>>;

    fn query(
        &self,
        ctx: Context,
        storage: &dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        msg: &[u8],
    ) -> VmResult<GenericResult<Json>>;

    fn before_tx(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        tx: Tx,
    ) -> VmResult<GenericResult<Response>>;

    fn after_tx(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        tx: Tx,
    ) -> VmResult<GenericResult<Response>>;

    fn handle_fee(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        tx: Tx,
        outcome: Outcome,
    ) -> VmResult<GenericResult<Response>>;

    fn bank_execute(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        msg: BankMsg,
    ) -> VmResult<GenericResult<Response>>;

    fn bank_query(
        &self,
        ctx: Context,
        storage: &dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        msg: BankQuery,
    ) -> VmResult<GenericResult<BankQueryResponse>>;

    fn cron_execute(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
    ) -> VmResult<GenericResult<Response>>;
}

// Trait alias is unstable:
// https://doc.rust-lang.org/beta/unstable-book/language-features/trait-alias.html
// So we define boxed traits as a workaround.

pub type InstantiateFn<M, E> = Box<dyn Fn(MutableCtx, M) -> Result<Response, E> + Send + Sync>;

pub type ExecuteFn<M, E> = Box<dyn Fn(MutableCtx, M) -> Result<Response, E> + Send + Sync>;

pub type MigrateFn<M, E> = Box<dyn Fn(MutableCtx, M) -> Result<Response, E> + Send + Sync>;

pub type ReceiveFn<E> = Box<dyn Fn(MutableCtx) -> Result<Response, E> + Send + Sync>;

pub type ReplyFn<M, E> = Box<dyn Fn(SudoCtx, M, SubMsgResult) -> Result<Response, E> + Send + Sync>;

pub type QueryFn<M, E> = Box<dyn Fn(ImmutableCtx, M) -> Result<Json, E> + Send + Sync>;

pub type BeforeTxFn<E> = Box<dyn Fn(AuthCtx, Tx) -> Result<Response, E> + Send + Sync>;

pub type AfterTxFn<E> = Box<dyn Fn(AuthCtx, Tx) -> Result<Response, E> + Send + Sync>;

pub type HandleFeeFn<E> = Box<dyn Fn(SudoCtx, Tx, Outcome) -> Result<Response, E> + Send + Sync>;

pub type BankExecuteFn<E> = Box<dyn Fn(SudoCtx, BankMsg) -> Result<Response, E> + Send + Sync>;

pub type BankQueryFn<E> = Box<dyn Fn(ImmutableCtx, BankQuery) -> Result<BankQueryResponse, E> + Send + Sync>;

pub type CronExecuteFn<E> = Box<dyn Fn(SudoCtx) -> Result<Response, E> + Send + Sync>;
