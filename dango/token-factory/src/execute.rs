use {
    crate::{ADMINS, CONFIG},
    anyhow::{bail, ensure},
    dango_account_factory::ACCOUNTS_BY_USER,
    dango_types::{
        account_factory::Username,
        bank::{self, Metadata},
        config::ACCOUNT_FACTORY_KEY,
        taxman,
        token_factory::{Config, ExecuteMsg, InstantiateMsg, NAMESPACE},
    },
    grug::{Addr, Coins, Denom, Inner, Message, MutableCtx, Part, Response, Uint128},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    CONFIG.save(ctx.storage, &msg.config)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::Configure { new_cfg } => configure(ctx, new_cfg),
        ExecuteMsg::Create {
            username,
            subdenom,
            admin,
            metadata,
        } => create(ctx, subdenom, username, admin, metadata),
        ExecuteMsg::Mint { denom, to, amount } => mint(ctx, denom, to, amount),
        ExecuteMsg::Burn {
            denom,
            from,
            amount,
        } => burn(ctx, denom, from, amount),
    }
}

fn configure(ctx: MutableCtx, new_cfg: Config) -> anyhow::Result<Response> {
    let cfg = ctx.querier.query_config()?;

    ensure!(
        ctx.sender == cfg.owner,
        "only the chain owner can update denom creation fee"
    );

    CONFIG.save(ctx.storage, &new_cfg)?;

    Ok(Response::new())
}

fn create(
    ctx: MutableCtx,
    subdenom: Denom,
    username: Option<Username>,
    admin: Option<Addr>,
    metadata: Option<Metadata>,
) -> anyhow::Result<Response> {
    // If the sender has chosen to use a username as the sub-namespace, ensure
    // the sender is associated with the username.
    // Otherwise, use the sender's address as the sub-namespace.
    let subnamespace = if let Some(username) = username {
        let account_factory = ctx.querier.query_app_config(ACCOUNT_FACTORY_KEY)?;

        if ctx
            .querier
            .query_wasm_raw(
                account_factory,
                ACCOUNTS_BY_USER.path((&username, ctx.sender)),
            )?
            .is_none()
        {
            bail!(
                "sender {} isn't associated with username `{username}`",
                ctx.sender,
            );
        }

        // A username is necessarily a valid denom part, so use uncheck here.
        Part::new_unchecked(username.into_inner())
    } else {
        // Same with address - necessarily a valid denom part.
        Part::new_unchecked(ctx.sender.to_string())
    };

    // Ensure the sender has paid the correct amount of fee.
    // If there's a non-zero fee, forward it to the taxman.
    let fee_msg = {
        let factory_cfg = CONFIG.load(ctx.storage)?;

        if let Some(fee) = factory_cfg.token_creation_fee {
            let expect = fee.into_inner();
            let actual = ctx.funds.into_one_coin()?;

            ensure!(
                actual == expect,
                "incorrect denom creation fee! expecting {expect}, got {actual}"
            );

            let cfg = ctx.querier.query_config()?;

            Some(Message::execute(
                cfg.taxman,
                &taxman::ExecuteMsg::Pay { payer: ctx.sender },
                actual,
            )?)
        } else {
            None
        }
    };

    // Ensure the denom hasn't already been created.
    let denom = {
        let denom = subdenom.prepend(&[&NAMESPACE, &subnamespace])?;
        let admin = admin.unwrap_or(ctx.sender);

        ensure!(
            !ADMINS.has(ctx.storage, &denom),
            "denom `{denom}` already exists"
        );

        ADMINS.save(ctx.storage, &denom, &admin)?;

        denom
    };

    // Optionally set the token's metadata.
    let metadata_msg = if let Some(metadata) = metadata {
        let cfg = ctx.querier.query_config()?;

        Some(Message::execute(
            cfg.bank,
            &bank::ExecuteMsg::SetMetadata { denom, metadata },
            Coins::new(),
        )?)
    } else {
        None
    };

    Ok(Response::new()
        .may_add_message(fee_msg)
        .may_add_message(metadata_msg))
}

fn mint(ctx: MutableCtx, denom: Denom, to: Addr, amount: Uint128) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ADMINS.load(ctx.storage, &denom)?,
        "sender isn't the admin of denom `{denom}`"
    );

    let cfg = ctx.querier.query_config()?;

    Ok(Response::new().add_message(Message::execute(
        cfg.bank,
        &bank::ExecuteMsg::Mint { to, denom, amount },
        Coins::new(),
    )?))
}

fn burn(ctx: MutableCtx, denom: Denom, from: Addr, amount: Uint128) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ADMINS.load(ctx.storage, &denom)?,
        "sender isn't the admin of denom `{denom}`"
    );

    let cfg = ctx.querier.query_config()?;

    Ok(Response::new().add_message(Message::execute(
        cfg.bank,
        &bank::ExecuteMsg::Burn {
            from,
            denom,
            amount,
        },
        Coins::new(),
    )?))
}
