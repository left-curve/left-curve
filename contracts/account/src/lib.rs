use {
    anyhow::{bail, ensure},
    cw_std::{
        cw_serde, entry_point, to_json, BeforeTxCtx, Binary, ExecuteCtx, InstantiateCtx, Item,
        QueryCtx, Response, Tx,
    },
};

const PUBKEY:   Item<PubKey> = Item::new("pk");
const SEQUENCE: Item<u64>    = Item::new("seq");

#[cw_serde]
pub struct InstantiateMsg {
    pubkey: PubKey,
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
    pubkey:   PubKey,
    sequence: u64,
}

#[cw_serde]
pub enum PubKey {
    Secp256k1(Binary),
    Secp256r1(Binary),
}

#[entry_point]
pub fn instantiate(ctx: InstantiateCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    PUBKEY.save(ctx.store, &msg.pubkey)?;
    SEQUENCE.save(ctx.store, &0)?;

    Ok(Response::new())
}

pub fn before_tx(ctx: BeforeTxCtx, tx: Tx) -> anyhow::Result<Response> {
    let pubkey = PUBKEY.load(ctx.store)?;
    let mut sequence = SEQUENCE.load(ctx.store)?;

    // prepare the hash that is expected to have been signed
    // msg_hash := blake3(json(tx) | utf8(chain_id) | bigendian(sequence))
    let tx_bytes = to_json(&tx)?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(tx_bytes.as_ref());
    hasher.update(ctx.block.chain_id.as_bytes());
    hasher.update(&sequence.to_be_bytes());

    // verify the signature
    // skip if it's simulate mode
    if !ctx.simulate {
        match pubkey {
            PubKey::Secp256k1(bytes) => {
                todo!()
            },
            PubKey::Secp256r1(bytes) => {
                todo!()
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
