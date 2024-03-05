//! Specifications:
//! https://github.com/cosmos/ibc/tree/main/spec/client/ics-006-solo-machine-client
//!
//! Go implementation:
//! https://github.com/cosmos/ibc-go/tree/v8.1.0/modules/light-clients/06-solomachine
//!
//! This implementation does not follow the ICS-06 specification. In the spec,
//! the client state is only updated when changing the public key or diversifier;
//! for verifications, a signature is used as the proof. This is a poor design,
//! because as part of the verification you'd have to increment the sequence in
//! order to prevent replays attacks. This means the verify functions become
//! state-mutative. This is a different behavior from every other client type.
//!
//! Technically the client spec (ICS-02) doesn't say verifications can't mutate
//! state, but doing so is just weird. A verification is supposed be a query,
//! and a query shouldn't mutate state.
//!
//! In our case, if we want to allow state mutations in the verify functions,
//! the `Context` needs to have a `&mut dyn Storage` instead of a `&dyn Storage`
//! which is a big change with security implications. It's just not worth it to
//! make this big of a change just to accommendate the need of this one client.
//!
//! Instead, we make it such that each header is associated with a key-value pair
//! (the `Record`). Each client update, the record is updated. For verification,
//! we just check whether the user-provided record matches the one in the header.
//! I think, this is a much cleaner design.
//!
//! We also made the following changes:
//!
//! - removed the `diversifier` string
//! - removed timestamp
//! - removed the ability to change the public key
//! - only support Secp256k1 public keys
//!
//! The solo machine is only intended for dev purposes, so we want to keep it
//! simple and trim all the features we don't need. If you need these features
//! please let us know.

#[cfg(not(feature = "library"))]
use cw_std::entry_point;
use {
    anyhow::{bail, ensure},
    cw_std::{
        cw_derive, from_json, hash, to_borsh, to_json, Api, Binary, ExecuteCtx,
        IbcClientExecuteMsg, IbcClientQueryMsg, IbcClientQueryResponse, IbcClientStateResponse,
        IbcClientStatus, InstantiateCtx, Item, QueryCtx, Response, StdResult,
    },
};

pub const CLIENT_STATE: Item<ClientState> = Item::new("client_state");
pub const CONSENSUS_STATE: Item<ConsensusState> = Item::new("consensus_state");

#[cw_derive(serde, borsh)]
pub struct ConsensusState {
    /// Secp256k1 public key for this solo machine.
    pub public_key: Binary,
    /// The total number of times the client state has been upated.When a new
    /// client is created, this is set to 0. Each time `ibc_update_client` is
    /// called, this is incremented by 1.
    ///
    /// The sequence is included in the `SignBytes` as replay protection.
    pub sequence: u64,
    /// An arbitrary piece of data associated with the current sequence.
    pub record: Option<Record>,
}

#[cw_derive(serde, borsh)]
pub struct ClientState {
    /// Client status is set to `Frozen` on misbehavior, otherwise `Active`.
    /// The solo machine client never expires.
    pub status: IbcClientStatus,
}

#[cw_derive(serde)]
pub struct Header {
    /// The key holder must sign the SHA-256 hash of the Borsh encoding of `SignBytes`.
    pub signature: Binary,
    /// Record for the new client state.
    pub record: Option<Record>,
}

/// A solo machine has committed a misbehavior if the key signs two different
/// headers at the same sequence.
#[cw_derive(serde)]
pub struct Misbehavior {
    pub sequence: u64,
    pub header_one: Header,
    pub header_two: Header,
}

/// A key-value pair.
#[cw_derive(serde, borsh)]
pub struct Record {
    pub path: Binary,
    pub data: Binary,
}

/// In order to update the client, the public key must sign the Borsh encoding
/// of this struct.
#[cw_derive(borsh)]
pub struct SignBytes {
    pub sequence: u64,
    pub record: Option<Record>,
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_client_create(
    ctx: InstantiateCtx,
    client_state: Binary,
    consensus_state: Binary,
) -> anyhow::Result<Response> {
    let client_state: ClientState = from_json(client_state)?;
    let consensus_state: ConsensusState = from_json(consensus_state)?;

    ensure!(client_state.status == IbcClientStatus::Active, "new client must be active");
    ensure!(consensus_state.sequence == 0, "sequence must start from zero");

    CLIENT_STATE.save(ctx.store, &client_state)?;
    CONSENSUS_STATE.save(ctx.store, &consensus_state)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_client_execute(ctx: ExecuteCtx, msg: IbcClientExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        IbcClientExecuteMsg::Update {
            header,
        } => update(ctx, header),
        IbcClientExecuteMsg::UpdateOnMisbehavior {
            misbehavior,
        } => update_on_misbehavior(ctx, misbehavior),
    }
}

pub fn update(ctx: ExecuteCtx, header: Binary) -> anyhow::Result<Response> {
    let header: Header = from_json(header)?;
    let client_state = CLIENT_STATE.load(ctx.store)?;
    let mut consensus_state = CONSENSUS_STATE.load(ctx.store)?;

    ensure!(
        client_state.status == IbcClientStatus::Active,
        "cannot upgrade client with status {:?}",
        client_state.status
    );

    verify_signature(&ctx, &consensus_state.public_key, consensus_state.sequence, &header)?;

    consensus_state.record = header.record;
    consensus_state.sequence += 1;

    CONSENSUS_STATE.save(ctx.store, &consensus_state)?;

    Ok(Response::new())
}

pub fn update_on_misbehavior(ctx: ExecuteCtx, misbehavior: Binary) -> anyhow::Result<Response> {
    let misbehavior: Misbehavior = from_json(misbehavior)?;
    let mut client_state = CLIENT_STATE.load(ctx.store)?;
    let consensus_state = CONSENSUS_STATE.load(ctx.store)?;

    ensure!(
        misbehavior.header_one != misbehavior.header_two,
        "misbehavior headers cannot be equal"
    );

    verify_signature(
        &ctx,
        &consensus_state.public_key,
        misbehavior.sequence,
        &misbehavior.header_one,
    )?;
    verify_signature(
        &ctx,
        &consensus_state.public_key,
        misbehavior.sequence,
        &misbehavior.header_two,
    )?;

    client_state.status = IbcClientStatus::Frozen;

    CLIENT_STATE.save(ctx.store, &client_state)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_client_query(
    ctx: QueryCtx,
    msg: IbcClientQueryMsg,
) -> anyhow::Result<IbcClientQueryResponse> {
    match msg {
        IbcClientQueryMsg::State {} => query_state(ctx).map(IbcClientQueryResponse::State),
        // solo machine does not utilize the height and delay period pamameters
        // for membership verification, per ICS-06 spec. our implementation also
        // does not use the proof.
        IbcClientQueryMsg::VerifyMembership {
            path,
            data,
            ..
        } => verify_membership(ctx, path, data).map(|_| IbcClientQueryResponse::VerifyMembership),
        // solo machine does not utilize the height and delay period parameters
        // for non-membership verification, per ICS-06 spec.
        IbcClientQueryMsg::VerifyNonMembership {
            path,
            ..
        } => verify_non_membership(ctx, path).map(|_| IbcClientQueryResponse::VerifyNonMembership),
    }
}

pub fn query_state(ctx: QueryCtx) -> anyhow::Result<IbcClientStateResponse> {
    let client_state = CLIENT_STATE.load(ctx.store)?;
    let consensus_state = CONSENSUS_STATE.load(ctx.store)?;
    Ok(IbcClientStateResponse {
        client_state: to_json(&client_state)?,
        consensus_state: to_json(&consensus_state)?,
    })
}

pub fn verify_membership(ctx: QueryCtx, path: Binary, data: Binary) -> anyhow::Result<()> {
    let client_state = CLIENT_STATE.load(ctx.store)?;
    let consensus_state = CONSENSUS_STATE.load(ctx.store)?;

    // if the client is frozen due to a misbehavior, then its state is not
    // trustworthy. all verifications should fail in this case.
    ensure!(client_state.status != IbcClientStatus::Frozen, "client is frozen due to misbehavior");

    // a record must exist for the current sequence, and its path and data must
    // both match the given values.
    let Some(record) = consensus_state.record else {
        bail!("expecting membership but record does not exist");
    };

    ensure!(
        record.path == path,
        "record exists but path does not match: {} != {path}",
        record.path
    );
    ensure!(
        record.data == data,
        "record exists but data does not match: {} != {data}",
        record.data
    );

    Ok(())
}

pub fn verify_non_membership(ctx: QueryCtx, path: Binary) -> anyhow::Result<()> {
    let client_state = CLIENT_STATE.load(ctx.store)?;
    let consensus_state = CONSENSUS_STATE.load(ctx.store)?;

    // if the client is frozen due to a misbehavior, then its state is not
    // trustworthy. all verifications should fail in this case.
    ensure!(client_state.status != IbcClientStatus::Frozen, "client is frozen due to misbehavior");

    // we're verifying non-membership now, so if the record exists, the path
    // must not match, otherwise it's a membership.
    if let Some(record) = consensus_state.record {
        ensure!(record.path != path, "expecting non-membership but record exists");
    }

    Ok(())
}

#[inline]
fn verify_signature(
    api: &dyn Api,
    public_key: &[u8],
    sequence: u64,
    header: &Header,
) -> StdResult<()> {
    let sign_bytes = SignBytes {
        sequence,
        record: header.record.clone(), // TODO: avoid this cloning
    };
    let sign_bytes_hash = hash(to_borsh(&sign_bytes)?);
    api.secp256k1_verify(&sign_bytes_hash, &header.signature, public_key)
}
