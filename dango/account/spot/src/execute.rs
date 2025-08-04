use {
    anyhow::ensure,
    dango_auth::authenticate_tx,
    dango_types::{DangoQuerier, account::spot::InstantiateMsg, bank},
    grug::{
        AuthCtx, AuthResponse, Coins, GENESIS_BLOCK_HEIGHT, Message, MutableCtx, QuerierExt,
        Response, StdResult, SubMessage, SubMsgResult, SudoCtx, Tx,
    },
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    // Only the account factory can create new accounts.
    ensure!(
        ctx.sender == ctx.querier.query_account_factory()?,
        "you don't have the right, O you don't have the right"
    );

    // Always claim orphaned transfers from the bank, regardless of whether a
    // minimum deposit is required.
    // However, skip this during genesis. Because during the genesis sequence,
    // the bank contract doesn't exist yet at this point.
    Ok(
        Response::new().may_add_submessage(if ctx.block.height > GENESIS_BLOCK_HEIGHT {
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

        _res.into_result().should_succeed();
    }

    if minimum_deposit.is_non_empty() {
        let balances = ctx.querier.query_balances(ctx.contract, None, None)?;

        ensure!(
            minimum_deposit
                .iter()
                .any(|coin| balances.amount_of(coin.denom) >= *coin.amount),
            "minimum deposit not satisfied! requiring any of: {minimum_deposit}, got: {balances}"
        );
    }

    Ok(Response::new())
}
