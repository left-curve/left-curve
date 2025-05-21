use {
    anyhow::ensure,
    dango_auth::authenticate_tx,
    dango_types::{DangoQuerier, account::spot::InstantiateMsg, bank},
    grug::{
        AuthCtx, AuthResponse, Coins, Message, MutableCtx, QuerierExt, Response, StdResult,
        SubMessage, SubMsgResult, SudoCtx, Tx,
    },
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    // Only the account factory can create new accounts.
    ensure!(
        ctx.sender == ctx.querier.query_account_factory()?,
        "you don't have the right, O you don't have the right"
    );

    Ok(
        Response::new().may_add_submessage(if msg.minimum_deposit.is_non_empty() {
            let bank = ctx.querier.query_bank()?;
            let gateway = ctx.querier.query_gateway()?;

            Some(SubMessage::reply_on_success(
                Message::execute(
                    bank,
                    &bank::ExecuteMsg::RecoverTransfer {
                        sender: gateway,
                        recipient: ctx.contract,
                    },
                    Coins::default(),
                )?,
                &msg.minimum_deposit,
            )?)
        } else {
            None
        }),
    )
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn authenticate(ctx: AuthCtx, tx: Tx) -> anyhow::Result<AuthResponse> {
    authenticate_tx(ctx, tx, None)?;

    Ok(AuthResponse::new().request_backrun(false))
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn receive(_ctx: MutableCtx) -> StdResult<Response> {
    // Do nothing, accept all transfers.
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn reply(ctx: SudoCtx, minimum_deposit: Coins, _res: SubMsgResult) -> anyhow::Result<Response> {
    #[cfg(debug_assertions)]
    {
        use grug::ResultExt;

        _res.should_succeed();
    }

    let balances = ctx.querier.query_balances(ctx.contract, None, None)?;

    ensure!(
        minimum_deposit
            .into_iter()
            .any(|coin| balances.amount_of(&coin.denom) >= coin.amount),
        "minumum deposit not satisfied!"
    );

    Ok(Response::new())
}
