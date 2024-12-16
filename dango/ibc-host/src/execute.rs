use {
    crate::CLIENT_IMPLS,
    anyhow::ensure,
    dango_types::ibc::host::{ClientType, ExecuteMsg, InstantiateMsg},
    grug::{Addr, MutableCtx, Response, StdResult},
    std::collections::BTreeMap,
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> StdResult<Response> {
    for (client_type, client_impl) in msg.client_impls {
        CLIENT_IMPLS.save(ctx.storage, client_type, &client_impl)?;
    }

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::RegisterClients(new_client_impls) => register_clients(ctx, new_client_impls),
    }
}

fn register_clients(
    ctx: MutableCtx,
    new_client_impls: BTreeMap<ClientType, Addr>,
) -> anyhow::Result<Response> {
    let cfg = ctx.querier.query_config()?;

    ensure!(
        ctx.sender == cfg.owner,
        "only the owner can register clients"
    );

    for (client_type, new_client_impl) in new_client_impls {
        CLIENT_IMPLS.save(ctx.storage, client_type, &new_client_impl)?;
    }

    Ok(Response::new())
}
