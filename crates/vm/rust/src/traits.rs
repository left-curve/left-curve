//! TODO: This file is named "traits" but it includes types instead of traits,
//! because we need trait aliases which is unstable:
//! https://doc.rust-lang.org/beta/unstable-book/language-features/trait-alias.html
//! Using boxes is a temporary workaround.

use {
    grug_types::{
        Api, BankQueryMsg, BankQueryResponse, Context, Empty, GenericResult, Json, Querier,
        Response, StdError, Storage, SubMsgResult, TransferMsg, Tx,
    },
    grug_wasm::{AuthCtx, ImmutableCtx, MutableCtx, SudoCtx},
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
}

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

pub type BankTransferFn<E = StdError> = Box<dyn Fn(SudoCtx, TransferMsg) -> Result<Response, E> + Send + Sync>;

pub type BankQueryFn<E = StdError> = Box<dyn Fn(ImmutableCtx, BankQueryMsg) -> Result<BankQueryResponse, E> + Send + Sync>;
