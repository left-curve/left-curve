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
use grug::entry_point;
use {
    anyhow::{bail, ensure},
    grug::{
        from_json_value, grug_derive, hash, to_borsh_vec, to_json_value, Api, Binary,
        IbcClientStatus, IbcClientUpdateMsg, IbcClientVerifyMsg, ImmutableCtx, Item, Json,
        Response, StdResult, SudoCtx,
    },
};

pub const CLIENT_STATE: Item<ClientState> = Item::new("client_state");
pub const CONSENSUS_STATE: Item<ConsensusState> = Item::new("consensus_state");

#[grug_derive(serde, borsh)]
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

#[grug_derive(serde, borsh)]
pub struct ClientState {
    /// Client status is set to `Frozen` on misbehavior, otherwise `Active`.
    /// The solo machine client never expires.
    pub status: IbcClientStatus,
}

#[grug_derive(serde)]
pub struct Header {
    /// The key holder must sign the SHA-256 hash of the Borsh encoding of `SignBytes`.
    pub signature: Binary,
    /// Record for the new client state.
    pub record: Option<Record>,
}

/// A solo machine has committed a misbehavior if the key signs two different
/// headers at the same sequence.
#[grug_derive(serde)]
pub struct Misbehavior {
    pub sequence: u64,
    pub header_one: Header,
    pub header_two: Header,
}

#[grug_derive(serde)]
pub enum QueryMsg {
    /// Query the client and consensus states.
    /// Returns: StateResponse
    State {},
}

#[grug_derive(serde)]
pub struct StateResponse {
    pub client_state: ClientState,
    pub consensus_state: ConsensusState,
}

/// A key-value pair.
#[grug_derive(serde, borsh)]
pub struct Record {
    pub key: Binary,
    pub value: Binary,
}

/// In order to update the client, the public key must sign the Borsh encoding
/// of this struct.
#[grug_derive(borsh)]
pub struct SignBytes {
    pub sequence: u64,
    pub record: Option<Record>,
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_client_create(
    ctx: SudoCtx,
    client_state: Json,
    consensus_state: Json,
) -> anyhow::Result<Response> {
    let client_state: ClientState = from_json_value(client_state)?;
    let consensus_state: ConsensusState = from_json_value(consensus_state)?;

    ensure!(
        client_state.status == IbcClientStatus::Active,
        "new client must be active"
    );
    ensure!(
        consensus_state.sequence == 0,
        "sequence must start from zero"
    );

    CLIENT_STATE.save(ctx.storage, &client_state)?;
    CONSENSUS_STATE.save(ctx.storage, &consensus_state)?;

    Ok(Response::new().add_attribute("consensus_height", consensus_state.sequence))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_client_update(ctx: SudoCtx, msg: IbcClientUpdateMsg) -> anyhow::Result<Response> {
    match msg {
        IbcClientUpdateMsg::Update { header } => update(ctx, header),
        IbcClientUpdateMsg::UpdateOnMisbehavior { misbehavior } => {
            update_on_misbehavior(ctx, misbehavior)
        },
    }
}

pub fn update(ctx: SudoCtx, header: Json) -> anyhow::Result<Response> {
    let header: Header = from_json_value(header)?;
    let client_state = CLIENT_STATE.load(ctx.storage)?;
    let mut consensus_state = CONSENSUS_STATE.load(ctx.storage)?;

    ensure!(
        client_state.status == IbcClientStatus::Active,
        "cannot upgrade client with status {:?}",
        client_state.status
    );

    verify_signature(
        ctx.api,
        &consensus_state.public_key,
        consensus_state.sequence,
        &header,
    )?;

    consensus_state.record = header.record;
    consensus_state.sequence += 1;

    CONSENSUS_STATE.save(ctx.storage, &consensus_state)?;

    Ok(Response::new().add_attribute("consensus_height", consensus_state.sequence))
}

pub fn update_on_misbehavior(ctx: SudoCtx, misbehavior: Json) -> anyhow::Result<Response> {
    let misbehavior: Misbehavior = from_json_value(misbehavior)?;
    let mut client_state = CLIENT_STATE.load(ctx.storage)?;
    let consensus_state = CONSENSUS_STATE.load(ctx.storage)?;

    ensure!(
        misbehavior.header_one != misbehavior.header_two,
        "misbehavior headers cannot be equal"
    );

    verify_signature(
        ctx.api,
        &consensus_state.public_key,
        misbehavior.sequence,
        &misbehavior.header_one,
    )?;
    verify_signature(
        ctx.api,
        &consensus_state.public_key,
        misbehavior.sequence,
        &misbehavior.header_two,
    )?;

    client_state.status = IbcClientStatus::Frozen;

    CLIENT_STATE.save(ctx.storage, &client_state)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_client_verify(ctx: ImmutableCtx, msg: IbcClientVerifyMsg) -> anyhow::Result<()> {
    match msg {
        // solo machine does not utilize the height and delay period pamameters
        // for membership verification, per ICS-06 spec. our implementation also
        // does not use the proof.
        IbcClientVerifyMsg::VerifyMembership { key, value, .. } => {
            verify_membership(ctx, key, value)
        },
        // solo machine does not utilize the height and delay period parameters
        // for non-membership verification, per ICS-06 spec.
        IbcClientVerifyMsg::VerifyNonMembership { key, .. } => verify_non_membership(ctx, key),
    }
}

pub fn verify_membership(ctx: ImmutableCtx, key: Binary, value: Binary) -> anyhow::Result<()> {
    let client_state = CLIENT_STATE.load(ctx.storage)?;
    let consensus_state = CONSENSUS_STATE.load(ctx.storage)?;

    // if the client is frozen due to a misbehavior, then its state is not
    // trustworthy. all verifications should fail in this case.
    ensure!(
        client_state.status != IbcClientStatus::Frozen,
        "client is frozen due to misbehavior"
    );

    // a record must exist for the current sequence, and its key and value must
    // both match the given values.
    let Some(record) = consensus_state.record else {
        bail!("expecting membership but record does not exist");
    };

    ensure!(
        record.key == key,
        "record exists but keys do not match: {} != {key}",
        record.key
    );
    ensure!(
        record.value == value,
        "record exists but values do not match: {} != {value}",
        record.value
    );

    Ok(())
}

pub fn verify_non_membership(ctx: ImmutableCtx, key: Binary) -> anyhow::Result<()> {
    let client_state = CLIENT_STATE.load(ctx.storage)?;
    let consensus_state = CONSENSUS_STATE.load(ctx.storage)?;

    // if the client is frozen due to a misbehavior, then its state is not
    // trustworthy. all verifications should fail in this case.
    ensure!(
        client_state.status != IbcClientStatus::Frozen,
        "client is frozen due to misbehavior"
    );

    // we're verifying non-membership now, so if the record exists, the key
    // must not match, otherwise it's a membership.
    if let Some(record) = consensus_state.record {
        ensure!(
            record.key != key,
            "expecting non-membership but record exists"
        );
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
    let sign_bytes_hash = hash(to_borsh_vec(&sign_bytes)?);
    api.secp256k1_verify(&sign_bytes_hash, &header.signature, public_key)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::State {} => to_json_value(&query_state(ctx)?),
    }
}

pub fn query_state(ctx: ImmutableCtx) -> StdResult<StateResponse> {
    Ok(StateResponse {
        client_state: CLIENT_STATE.load(ctx.storage)?,
        consensus_state: CONSENSUS_STATE.load(ctx.storage)?,
    })
}
