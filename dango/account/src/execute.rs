use {
    anyhow::{bail, ensure},
    dango_types::{
        account::{ExecuteMsg, InstantiateMsg},
        auth::AccountStatus,
    },
    grug_types::{AuthCtx, MutableCtx, QuerierExt, Response, Tx},
};

pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    dango_auth::create_account(ctx, msg.activate)?;

    Ok(Response::new())
}

pub fn authenticate(ctx: AuthCtx, tx: Tx) -> anyhow::Result<Response> {
    dango_auth::authenticate_tx(ctx, tx, None)?;

    Ok(Response::new())
}

pub fn receive(ctx: MutableCtx) -> anyhow::Result<Response> {
    dango_auth::receive_transfer(ctx)?;

    Ok(Response::new())
}

pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "you don't have the right, O you don't have the right"
    );

    let status = dango_auth::query_status(ctx.storage)?;

    match msg {
        ExecuteMsg::Freeze {} => {
            dango_auth::account::STATUS.save(ctx.storage, &AccountStatus::Frozen)?;
        },
        ExecuteMsg::Unfreeze {} => {
            if status != AccountStatus::Frozen {
                bail!("can only unfreeze a frozen account; current status: {status:?}");
            }
            dango_auth::account::STATUS.save(ctx.storage, &AccountStatus::Active)?;
        },
    }

    Ok(Response::new())
}
