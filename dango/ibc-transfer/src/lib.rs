use {
    dango_account_factory::ACCOUNTS,
    dango_types::{
        account_factory,
        config::AppConfig,
        ibc_transfer::{ExecuteMsg, InstantiateMsg},
    },
    grug::{Addr, Coins, Message, MutableCtx, Response, StdError, StdResult},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(_ctx: MutableCtx, _msg: InstantiateMsg) -> StdResult<Response> {
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> StdResult<Response> {
    reject_unintended_deposits(ctx.funds.clone(), msg.clone())?;
    match msg {
        ExecuteMsg::ReceiveTransfer { recipient } => receive_transfer(ctx, recipient),
    }
}

// reject_unintended_deposits returns error when funds are provided for messages
// not explicitly whitelisted.
fn reject_unintended_deposits(funds: Coins, msg: ExecuteMsg) -> StdResult<()> {
    match msg {
        ExecuteMsg::ReceiveTransfer { .. } => (),
        _ => {
            if !funds.is_empty() {
                return Err(StdError::invalid_coins(format!(
                    "unexpected funds: {}",
                    funds,
                )));
            }
        },
    }
    Ok(())
}

fn receive_transfer(ctx: MutableCtx, recipient: Addr) -> StdResult<Response> {
    let app_cfg: AppConfig = ctx.querier.query_app_config()?;

    // Query the factory to find whether the recipient exists:
    // - if yes, simply send the tokens to the accounts;
    // - if no, deposit the coins at the factory to be claimed later.
    // Use a raw instead of smart query to save on gas.
    let msg = if ctx
        .querier
        .query_wasm_raw(app_cfg.addresses.account_factory, ACCOUNTS.path(recipient))?
        .is_none()
    {
        Message::execute(
            app_cfg.addresses.account_factory,
            &account_factory::ExecuteMsg::Deposit { recipient },
            ctx.funds,
        )?
    } else {
        Message::transfer(recipient, ctx.funds)?
    };

    Ok(Response::new().add_message(msg))
}
