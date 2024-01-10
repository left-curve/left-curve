use {
    anyhow::ensure,
    cw_std::{
        cw_serde, entry_point, to_json, Addr, BeforeTxCtx, Binary, ExecuteCtx, InstantiateCtx,
        Item, Message, QueryCtx, Response, Tx,
    },
    sha2::{Digest, Sha256},
};

const PUBKEY:   Item<PubKey> = Item::new("pk");
const SEQUENCE: Item<u32>    = Item::new("seq");

#[cw_serde]
pub struct InstantiateMsg {
    pub pubkey: PubKey,
}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateKey {
        new_pubkey: PubKey,
    },
}

#[cw_serde]
pub enum QueryMsg {
    /// Query the state of the account, including its public key and sequence.
    /// Returns: StateResponse
    State {},
}

#[cw_serde]
pub struct StateResponse {
    pub pubkey:   PubKey,
    pub sequence: u32,
}

#[cw_serde]
pub enum PubKey {
    Secp256k1(Binary),
    Secp256r1(Binary),
}

/// Given details of a transaction, produce the bytes that the sender needs to
/// sign (hashed).
///
/// The bytes are defined as:
///
/// ```plain
/// bytes := blake3(json(msgs) | sender_addr | chain_id | sequence)
/// ```
///
/// where:
/// - `sender_addr` is a 32 bytes address of the sender;
/// - `chain_id` is the chain ID in UTF-8 encoding;
/// - `sequence` is the sender account's sequence in 32-bit big endian encoding.
///
/// TODO: json here is ambiguous, i.e. what padding and linebreak character to
/// use, the order of fields... elaborate it.
///
/// TODO: is it efficient to do hashing in the contract? maybe move this to the
/// host??
pub fn sign_bytes(
    msgs:     &[Message],
    sender:   &Addr,
    chain_id: &str,
    sequence: u32,
) -> anyhow::Result<[u8; 32]> {
    let mut hasher = Sha256::new();
    hasher.update(to_json(&msgs)?.as_ref());
    hasher.update(sender.as_ref());
    hasher.update(chain_id.as_bytes());
    hasher.update(&sequence.to_be_bytes());
    Ok(hasher.finalize().into())
}

#[entry_point]
pub fn instantiate(ctx: InstantiateCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    PUBKEY.save(ctx.store, &msg.pubkey)?;
    SEQUENCE.save(ctx.store, &0)?;

    Ok(Response::new())
}

#[entry_point]
pub fn before_tx(ctx: BeforeTxCtx, tx: Tx) -> anyhow::Result<Response> {
    let pubkey = PUBKEY.load(ctx.store)?;
    let mut sequence = SEQUENCE.load(ctx.store)?;

    // prepare the hash that is expected to have been signed
    let msg_hash = sign_bytes(&tx.msgs, &tx.sender, &ctx.block.chain_id, sequence)?;

    // verify the signature
    // skip if we are in simulate mode
    if !ctx.simulate {
        match &pubkey {
            PubKey::Secp256k1(bytes) => {
                ctx.secp256k1_verify(msg_hash, tx.credential.as_ref(), bytes.as_ref())?;
            },
            PubKey::Secp256r1(bytes) => {
                ctx.secp256r1_verify(msg_hash, tx.credential.as_ref(), bytes.as_ref())?;
            },
        }
    }

    // update sequence
    sequence += 1;
    SEQUENCE.save(ctx.store, &sequence)?;

    Ok(Response::new()
        .add_attribute("method", "before_tx")
        .add_attribute("next_sequence", sequence.to_string()))
}

#[entry_point]
pub fn execute(ctx: ExecuteCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::UpdateKey {
            new_pubkey,
        } => update_key(ctx, new_pubkey),
    }
}

#[entry_point]
pub fn query(ctx: QueryCtx, msg: QueryMsg) -> anyhow::Result<Binary> {
    match msg {
        QueryMsg::State {} => to_json(&query_state(ctx)?),
    }
}

pub fn update_key(ctx: ExecuteCtx, new_pubkey: PubKey) -> anyhow::Result<Response> {
    ensure!(ctx.sender == ctx.contract, "only the account itself can update key");
    // TODO: ensure new pubkey is valid?

    PUBKEY.save(ctx.store, &new_pubkey)?;

    Ok(Response::new())
}

pub fn query_state(ctx: QueryCtx) -> anyhow::Result<StateResponse> {
    Ok(StateResponse {
        pubkey:   PUBKEY.load(ctx.store)?,
        sequence: SEQUENCE.load(ctx.store)?,
    })
}
