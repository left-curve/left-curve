use {
    crate::state::WHITELISTED_DENOMS,
    anyhow::{bail, ensure, Ok},
    dango_auth::authenticate_tx,
    dango_types::{
        account_factory::QueryAccountRequest,
        bank,
        config::ACCOUNT_FACTORY_KEY,
        lending_pool::{ExecuteMsg, InstantiateMsg, NAMESPACE},
    },
    grug::{
        Addr, AuthCtx, AuthResponse, Coin, Coins, Denom, Inner, Message, MutableCtx, Part,
        Response, Tx,
    },
    std::str::FromStr,
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    // Store the whitelisted denoms.
    for denom in msg.whitelisted_denoms {
        WHITELISTED_DENOMS.insert(ctx.storage, denom)?;
    }

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn authenticate(ctx: AuthCtx, tx: Tx) -> anyhow::Result<AuthResponse> {
    authenticate_tx(ctx, tx, None, None)?;

    Ok(AuthResponse::new().request_backrun(false))
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn receive(_ctx: MutableCtx) -> anyhow::Result<Response> {
    // Reject all transfers.
    bail!("Can't send tokens to this contract. Use the `deposit` message instead.");
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::Deposit { recipient } => deposit(ctx, recipient),
        ExecuteMsg::Withdraw { recipient } => withdraw(ctx, recipient),
    }
}

/// Ensures that the sender's account is not a margin account.
fn ensure_sender_account_is_not_margin(ctx: &MutableCtx) -> anyhow::Result<()> {
    let account_factory: Addr = ctx.querier.query_app_config(ACCOUNT_FACTORY_KEY)?;
    ensure!(
        !ctx.querier
            .query_wasm_smart(account_factory, QueryAccountRequest {
                address: ctx.sender,
            })?
            .params
            .is_margin(),
        "Margin accounts can't deposit or withdraw"
    );
    Ok(())
}

pub fn deposit(ctx: MutableCtx, recipient: Option<Addr>) -> anyhow::Result<Response> {
    // Ensure margin accounts can't deposit
    ensure_sender_account_is_not_margin(&ctx)?;

    // For each deposited denom, ensure it's whitelisted and mint LP tokens.
    let cfg = ctx.querier.query_config()?;
    let mut msgs = vec![];
    for coin in ctx.funds.into_iter() {
        ensure!(
            WHITELISTED_DENOMS.has(ctx.storage, coin.denom.clone()),
            "Invalid denom"
        );

        let lp_denom = Denom::from_parts(vec![
            Part::from_str(NAMESPACE)?,
            Part::from_str("lp")?,
            Part::from_str(&coin.denom.to_string())?,
        ])?;

        msgs.push(Message::execute(
            cfg.bank,
            &bank::ExecuteMsg::Mint {
                to: recipient.unwrap_or(ctx.sender),
                denom: lp_denom,
                amount: coin.amount,
            },
            Coins::new(),
        )?);
    }

    Ok(Response::new().add_messages(msgs))
}

pub fn withdraw(ctx: MutableCtx, recipient: Option<Addr>) -> anyhow::Result<Response> {
    // Ensure margin accounts can't withdraw
    ensure_sender_account_is_not_margin(&ctx)?;

    // Unwrap the recipient
    let recipient = recipient.unwrap_or(ctx.sender);

    // Ensure there are funds to withdraw
    ensure!(!ctx.funds.is_empty(), "No funds to withdraw");

    let cfg = ctx.querier.query_config()?;
    let mut msgs = vec![];
    let mut withdrawn = Coins::new();
    for coin in ctx.funds.into_iter() {
        // Ensure only LP tokens are sent
        ensure!(
            coin.denom.inner().len() == 3
                && coin.denom.namespace().is_some_and(|ns| **ns == NAMESPACE)
                && coin.denom.inner().to_vec()[1].as_str() == "lp",
            "Invalid denom"
        );

        // Add msg to send the underlying tokens to the recipient
        let underlying_denom = Denom::from_parts(vec![coin.denom.inner()[2].clone()])?;
        let amount = coin.amount;
        withdrawn.insert(Coin::new(underlying_denom, amount)?)?;

        // Burn the LP tokens
        msgs.push(Message::execute(
            cfg.bank,
            &bank::ExecuteMsg::Burn {
                from: ctx.contract,
                denom: coin.denom,
                amount,
            },
            Coins::new(),
        )?);
    }

    // Transfer the underlying tokens to the recipient
    msgs.push(Message::transfer(recipient, withdrawn)?);

    Ok(Response::new().add_messages(msgs))
}
