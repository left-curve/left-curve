use {
    dango_account_factory::ACCOUNTS,
    dango_types::{
        account_factory,
        config::ACCOUNT_FACTORY_KEY,
        ibc_transfer::{ExecuteMsg, InstantiateMsg},
    },
    grug::{Addr, Message, MutableCtx, Response, StdResult},
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
    let factory = ctx.querier.query_app_config(ACCOUNT_FACTORY_KEY)?;

    // Query the factory to find whether the recipient exists:
    // - if yes, simply send the tokens to the accounts;
    // - if no, deposit the coins at the factory to be claimed later.
    // Use a raw instead of smart query to save on gas.
    let msg = if ctx
        .querier
        .query_wasm_raw(factory, ACCOUNTS.path(recipient))?
        .is_none()
    {
        Message::execute(
            factory,
            &account_factory::ExecuteMsg::Deposit { recipient },
            ctx.funds,
        )?
    } else {
        Message::transfer(recipient, ctx.funds)?
    };

    Ok(Response::new().add_message(msg))
}
