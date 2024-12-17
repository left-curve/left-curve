use {
    crate::{
        CLIENTS, CLIENT_REGISTRY, CLIENT_STATES, COMMITMENTS, CONSENSUS_STATES, NEXT_CLIENT_ID,
    },
    anyhow::ensure,
    dango_types::ibc::{
        self,
        host::{Client, ClientType, ExecuteMsg, InstantiateMsg},
    },
    grug::{Addr, Json, MutableCtx, Response, StdResult},
    ibc_union_spec::{ClientStatePath, ConsensusStatePath},
    std::collections::BTreeMap,
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> StdResult<Response> {
    for (client_type, client_impl) in msg.client_impls {
        CLIENT_REGISTRY.save(ctx.storage, client_type, &client_impl)?;
    }

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::RegisterClients(new_client_impls) => register_clients(ctx, new_client_impls),
        ExecuteMsg::CreateClient {
            client_type,
            client_state,
            consensus_state,
        } => create_client(ctx, client_type, client_state, consensus_state),
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
        CLIENT_REGISTRY.save(ctx.storage, client_type, &new_client_impl)?;
    }

    Ok(Response::new())
}

fn create_client(
    ctx: MutableCtx,
    client_type: ClientType,
    client_state: Json,
    consensus_state: Json,
) -> anyhow::Result<Response> {
    let client_impl = CLIENT_REGISTRY.load(ctx.storage, client_type)?;
    let (client_id, _) = NEXT_CLIENT_ID.increment(ctx.storage)?;

    let res =
        ctx.querier
            .query_wasm_smart(client_impl, ibc::client::QueryVerifyCreationRequest {
                client_state: client_state.clone(),
                consensus_state: consensus_state.clone(),
            })?;

    CLIENTS.save(ctx.storage, client_id, &Client {
        client_type,
        client_impl,
    })?;

    CLIENT_STATES.save(ctx.storage, client_id, &client_state)?;

    CONSENSUS_STATES.save(
        ctx.storage,
        (client_id, res.latest_height),
        &consensus_state,
    )?;

    COMMITMENTS.save(
        ctx.storage,
        ClientStatePath { client_id }.key().get(),
        &ctx.api.keccak256(&res.raw_client_state),
    )?;

    COMMITMENTS.save(
        ctx.storage,
        ConsensusStatePath {
            client_id,
            height: res.latest_height,
        }
        .key()
        .get(),
        &ctx.api.keccak256(&res.raw_consensus_state),
    )?;

    Ok(Response::new())
}
