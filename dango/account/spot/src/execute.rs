use {
    anyhow::ensure,
    dango_auth::authenticate_tx,
    dango_types::{account::spot::InstantiateMsg, bank, DangoQuerier},
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
        Response::new().may_add_submessage(if msg.at_least.is_non_empty() {
            let bank_addr = ctx.querier.query_bank()?;
            let warp = ctx.querier.query_warp()?;

            Some(SubMessage::reply_on_success(
                Message::execute(
                    bank_addr,
                    &bank::ExecuteMsg::ClaimPendingTransfer {
                        sender: warp,
                        recipient: ctx.contract,
                    },
                    Coins::default(),
                )?,
                &msg.at_least,
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
pub fn reply(ctx: SudoCtx, minimum_amount: Coins, _res: SubMsgResult) -> anyhow::Result<Response> {
    let balances = ctx.querier.query_balances(ctx.contract, None, None)?;

    ensure!(
        minimum_amount
            .into_iter()
            .any(|minimum_coin| balances.amount_of(&minimum_coin.denom) >= minimum_coin.amount),
        "minumum required amount not met"
    );

    Ok(Response::new())
}
