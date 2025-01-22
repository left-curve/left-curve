use {
    crate::{LOCAL_DOMAIN, MAILBOX, REPLAY_PROTECTIONS, STORAGE_LOCATIONS, VALIDATORS},
    anyhow::ensure,
    grug::{
        Addr, Empty, Hash256, HashExt, HexBinary, HexByteArray, ImmutableCtx, MutableCtx,
        QuerierExt, Response,
    },
    hyperlane_types::{
        eip191_hash, mailbox,
        va::{Announcement, ExecuteMsg, Initialized, InstantiateMsg},
    },
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    let local_domain = ctx
        .querier
        .query_wasm_smart(msg.mailbox, mailbox::QueryConfigRequest {})?
        .local_domain;

    MAILBOX.save(ctx.storage, &msg.mailbox)?;
    LOCAL_DOMAIN.save(ctx.storage, &local_domain)?;
    Ok(
        Response::new().add_event("init-validator-announce", &Initialized {
            creator: ctx.sender,
            mailbox: msg.mailbox,
            local_domain,
        })?,
    )
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::Announce {
            validator,
            signature,
            storage_location,
        } => announce(ctx, validator, signature, storage_location),
    }
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn migrate(_ctx: ImmutableCtx, _msg: Empty) -> anyhow::Result<Response> {
    Ok(Response::new())
}

fn announce(
    ctx: MutableCtx,
    validator: HexBinary,
    signature: HexBinary,
    storage_location: String,
) -> anyhow::Result<Response> {
    ensure!(validator.len() == 20, "length should be 20");

    // Check replay protection.
    let replay_id = replay_hash(&validator, &storage_location);
    ensure!(
        !REPLAY_PROTECTIONS.has(ctx.storage, replay_id),
        "replay protection triggered"
    );
    REPLAY_PROTECTIONS.insert(ctx.storage, replay_id)?;

    // Make announcement digest.
    let local_domain = LOCAL_DOMAIN.load(ctx.storage)?;
    let mailbox_addr = MAILBOX.load(ctx.storage)?;

    let message_hash = eip191_hash(announcement_hash(
        domain_hash(local_domain, mailbox_addr.into()).to_vec(),
        &storage_location,
    ));

    // Recover pubkey from signature & verify.
    let pubkey = ctx.api.secp256k1_pubkey_recover(
        &message_hash,
        &signature,
        // We subs 27 according to this - https://eips.ethereum.org/EIPS/eip-155
        signature[64] - 27,
        false,
    )?;
    ensure!(HexBinary::from(pubkey) == validator, "pubkey mismatch");

    let validator_hexbyte = HexByteArray::from_inner(validator.to_vec().try_into().unwrap());

    // Save validator if not saved yet.
    if !VALIDATORS.has(ctx.storage, validator_hexbyte) {
        VALIDATORS.insert(ctx.storage, validator_hexbyte)?;
    }

    // Append storage_locations.
    let mut storage_locations = STORAGE_LOCATIONS
        .may_load(ctx.storage, validator_hexbyte)?
        .unwrap_or_default();
    storage_locations.push(storage_location.clone());
    STORAGE_LOCATIONS.save(ctx.storage, validator_hexbyte, &storage_locations)?;

    Ok(
        Response::new().add_event("validator-announcement", &Announcement {
            sender: ctx.sender,
            validator,
            storage_location,
        })?,
    )
}

fn replay_hash(validator: &HexBinary, storage_location: &str) -> Hash256 {
    [validator.to_vec(), storage_location.as_bytes().to_vec()]
        .concat()
        .keccak256()
}

fn domain_hash(local_domain: u32, mailbox: Addr) -> Hash256 {
    let mut bz = vec![];
    bz.append(&mut local_domain.to_be_bytes().to_vec());
    // left pad with zeroes
    let mut addr = [0u8; 32];
    addr[32 - mailbox.len()..].copy_from_slice(&mailbox);
    bz.append(&mut addr.to_vec());
    bz.append(&mut "HYPERLANE_ANNOUNCEMENT".as_bytes().to_vec());

    bz.keccak256()
}

fn announcement_hash(mut domain_hash: Vec<u8>, storage_location: &str) -> Hash256 {
    let mut bz = vec![];
    bz.append(&mut domain_hash);
    bz.append(&mut storage_location.as_bytes().to_vec());

    bz.keccak256()
}

// ----------------------------------- tests -----------------------------------
