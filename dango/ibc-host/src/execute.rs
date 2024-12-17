use {
    crate::{
        CLIENTS, CLIENT_REGISTRY, COMMITMENTS, NEXT_CLIENT_ID, RAW_CLIENT_STATES,
        RAW_CONSENSUS_STATES,
    },
    anyhow::ensure,
    dango_types::ibc::{
        self,
        events::{ClientCreated, ClientUpdated},
        host::{Client, ClientId, ClientType, ExecuteMsg, InstantiateMsg},
    },
    grug::{Addr, Json, MutableCtx, Response, StdResult},
    ibc_union_spec::{ClientStatePath, ConsensusStatePath},
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
        ExecuteMsg::RegisterClient {
            client_type,
            client_impl,
        } => register_client(ctx, client_type, client_impl),
        ExecuteMsg::CreateClient {
            client_type,
            client_state,
            consensus_state,
        } => create_client(ctx, client_type, client_state, consensus_state),
        ExecuteMsg::UpdateClient {
            client_id,
            client_message,
        } => update_client(ctx, client_id, client_message),
    }
}

fn register_client(
    ctx: MutableCtx,
    client_type: ClientType,
    client_impl: Addr,
) -> anyhow::Result<Response> {
    let cfg = ctx.querier.query_config()?;

    ensure!(
        ctx.sender == cfg.owner,
        "only the owner can register clients"
    );

    CLIENT_REGISTRY.save(ctx.storage, client_type, &client_impl)?;

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
                client_state,
                consensus_state,
            })?;

    CLIENTS.save(ctx.storage, client_id, &Client {
        client_type,
        client_impl,
    })?;

    RAW_CLIENT_STATES.save(ctx.storage, client_id, &res.raw_client_state)?;

    RAW_CONSENSUS_STATES.save(
        ctx.storage,
        (client_id, res.latest_height),
        &res.raw_consensus_state,
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

    Ok(Response::new().add_event("client_created", &ClientCreated {
        client_id,
        client_type,
        consensus_height: res.latest_height,
    })?)
}

fn update_client(
    ctx: MutableCtx,
    client_id: ClientId,
    client_message: Json,
) -> anyhow::Result<Response> {
    let client = CLIENTS.load(ctx.storage, client_id)?;

    let res = ctx.querier.query_wasm_smart(
        client.client_impl,
        ibc::client::QueryVerifyClientMessageRequest {
            client_id,
            client_message,
        },
    )?;

    RAW_CLIENT_STATES.save(ctx.storage, client_id, &res.raw_client_state)?;

    RAW_CONSENSUS_STATES.save(
        ctx.storage,
        (client_id, res.height),
        &res.raw_consensus_state,
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
            height: res.height,
        }
        .key()
        .get(),
        &ctx.api.keccak256(&res.raw_consensus_state),
    )?;

    Ok(Response::new().add_event("client_updated", &ClientUpdated {
        client_id,
        client_type: client.client_type,
        height: res.height,
    })?)
}
