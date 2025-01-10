use {
    dango_account_factory::ACCOUNTS,
    dango_types::{
        account_factory,
        ibc::transfer::{ExecuteMsg, InstantiateMsg},
        DangoQuerier,
    },
    grug::{Addr, Message, MutableCtx, QuerierCore, Response, StdResult},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(_ctx: MutableCtx, _msg: InstantiateMsg) -> StdResult<Response> {
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::ReceiveTransfer { recipient } => receive_transfer(ctx, recipient),
    }
}

fn receive_transfer(ctx: MutableCtx, recipient: Addr) -> StdResult<Response> {
    let account_factory = ctx.querier.query_account_factory()?;

    // Query the factory to find whether the recipient exists:
    // - if yes, simply send the tokens to the accounts;
    // - if no, deposit the coins at the factory to be claimed later.
    // Use a raw instead of smart query to save on gas.
    let msg = if ctx
        .querier
        .query_wasm_raw(account_factory, ACCOUNTS.path(recipient))?
        .is_none()
    {
        Message::execute(
            account_factory,
            &account_factory::ExecuteMsg::Deposit { recipient },
            ctx.funds,
        )?
    } else {
        Message::transfer(recipient, ctx.funds)?
    };

    Ok(Response::new().add_message(msg))
}
