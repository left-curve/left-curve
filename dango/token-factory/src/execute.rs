use {
    crate::{CONFIG, DENOM_ADMINS},
    anyhow::{bail, ensure},
    dango_account_factory::ACCOUNTS_BY_USER,
    dango_types::{
        account_factory::Username,
        bank,
        config::ACCOUNT_FACTORY_KEY,
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
        } => create(ctx, subdenom, username, admin),
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

        username.to_string()
    } else {
        ctx.sender.to_string()
    };

    // Ensure the sender has paid the correct amount of fee.
    // Note: the logic here assumes the expected fee isn't zero, which we make
    // sure of during instantiation.
    {
        let cfg = CONFIG.load(ctx.storage)?;

        if let Some(fee) = cfg.denom_creation_fee {
            let expect = fee.into_inner();
            let actual = ctx.funds.into_one_coin()?;

            ensure!(
                actual == expect,
                "incorrect denom creation fee! expecting {expect}, got {actual}"
            );
        }
    }

    // Ensure the denom hasn't already been created.
    {
        let denom = {
            let mut parts = Vec::with_capacity(2 + subdenom.inner().len());
            parts.push(Part::new_unchecked(NAMESPACE));
            parts.push(Part::new_unchecked(subnamespace));
            parts.extend(subdenom.into_inner());

            Denom::from_parts(parts)?
        };

        let admin = admin.unwrap_or(ctx.sender);

        ensure!(
            !DENOM_ADMINS.has(ctx.storage, &denom),
            "denom `{denom}` already exists"
        );

        DENOM_ADMINS.save(ctx.storage, &denom, &admin)?;
    }

    Ok(Response::new())
}

fn mint(ctx: MutableCtx, denom: Denom, to: Addr, amount: Uint128) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == DENOM_ADMINS.load(ctx.storage, &denom)?,
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
        ctx.sender == DENOM_ADMINS.load(ctx.storage, &denom)?,
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
