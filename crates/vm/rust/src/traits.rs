// Skip formatting this entire file
// https://stackoverflow.com/questions/59247458/is-there-a-stable-way-to-tell-rustfmt-to-skip-an-entire-file
#![cfg_attr(rustfmt, rustfmt::skip)]

use grug_types::{
    Api, AuthCtx, BankMsg, BankQuery, BankQueryResponse, Context, Empty, GenericResult,
    ImmutableCtx, Json, MutableCtx, Querier, Response, StdError, Storage, SubMsgResult, SudoCtx,
    Tx,
};

pub trait Contract {
    fn instantiate(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        msg: Json,
    ) -> GenericResult<Response>;

    fn execute(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        msg: Json,
    ) -> GenericResult<Response>;

    fn migrate(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        msg: Json,
    ) -> GenericResult<Response>;

    fn receive(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
    ) -> GenericResult<Response>;

    fn reply(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        msg: Json,
        submsg_res: SubMsgResult,
    ) -> GenericResult<Response>;

    fn query(
        &self,
        ctx: Context,
        storage: &dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        msg: Json,
    ) -> GenericResult<Json>;

    fn before_tx(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        tx: Tx,
    ) -> GenericResult<Response>;

    fn after_tx(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        tx: Tx,
    ) -> GenericResult<Response>;

    fn before_block(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
    ) -> GenericResult<Response>;

    fn after_block(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
    ) -> GenericResult<Response>;

    fn bank_execute(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        msg: BankMsg,
    ) -> GenericResult<Response>;

    fn bank_query(
        &self,
        ctx: Context,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &dyn Querier,
        msg: BankQuery,
    ) -> GenericResult<BankQueryResponse>;
}

// Trait aliases are unstable:
// https://doc.rust-lang.org/beta/unstable-book/language-features/trait-alias.html
// So we define boxed traits as a workaround.

pub type InstantiateFn<M = Empty, E = StdError> = Box<dyn Fn(MutableCtx, M) -> Result<Response, E> + Send + Sync>;

pub type ExecuteFn<M = Empty, E = StdError> = Box<dyn Fn(MutableCtx, M) -> Result<Response, E> + Send + Sync>;

pub type MigrateFn<M = Empty, E = StdError> = Box<dyn Fn(MutableCtx, M) -> Result<Response, E> + Send + Sync>;

pub type ReceiveFn<E = StdError> = Box<dyn Fn(MutableCtx) -> Result<Response, E> + Send + Sync>;

pub type ReplyFn<M = Empty, E = StdError> = Box<dyn Fn(SudoCtx, M, SubMsgResult) -> Result<Response, E> + Send + Sync>;

pub type QueryFn<M = Empty, E = StdError> = Box<dyn Fn(ImmutableCtx, M) -> Result<Json, E> + Send + Sync>;

pub type BeforeTxFn<E = StdError> = Box<dyn Fn(AuthCtx, Tx) -> Result<Response, E> + Send + Sync>;

pub type AfterTxFn<E = StdError> = Box<dyn Fn(AuthCtx, Tx) -> Result<Response, E> + Send + Sync>;

pub type BeforeBlockFn<E = StdError> = Box<dyn Fn(SudoCtx) -> Result<Response, E> + Send + Sync>;

pub type AfterBlockFn<E = StdError> = Box<dyn Fn(SudoCtx) -> Result<Response, E> + Send + Sync>;

pub type BankExecuteFn<E = StdError> = Box<dyn Fn(SudoCtx, BankMsg) -> Result<Response, E> + Send + Sync>;

pub type BankQueryFn<E = StdError> = Box<dyn Fn(ImmutableCtx, BankQuery) -> Result<BankQueryResponse, E> + Send + Sync>;
