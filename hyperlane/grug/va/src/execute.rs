use {
    crate::{MAILBOX, REPLAY_PROTECTIONS, STORAGE_LOCATIONS, VALIDATORS},
    anyhow::ensure,
    grug::{
        Hash256, HashExt, HexByteArray, Inner, MutableCtx, Response, StdResult, StorageQuerier,
    },
    hyperlane_types::{
        domain_hash, eip191_hash,
        va::{EvtAnnouncement, EvtInitialize, ExecuteMsg, InstantiateMsg, VA_DOMAIN_KEY},
    },
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    MAILBOX.save(ctx.storage, &msg.mailbox)?;

    Ok(
        Response::new().add_event("init-validator-announce", &EvtInitialize {
            creator: ctx.sender,
            mailbox: msg.mailbox,
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

fn announce(
    ctx: MutableCtx,
    validator: HexByteArray<20>,
    signature: HexByteArray<65>,
    storage_location: String,
) -> anyhow::Result<Response> {
    // Check replay protection.
    let replay_id = replay_hash(validator, &storage_location);
    ensure!(
        !REPLAY_PROTECTIONS.has(ctx.storage, replay_id),
        "replay protection triggered"
    );
    REPLAY_PROTECTIONS.insert(ctx.storage, replay_id)?;

    // Make announcement digest.
    let mailbox_addr = MAILBOX.load(ctx.storage)?;

    let local_domain = ctx
        .querier
        .query_wasm_path(mailbox_addr, hyperlane_mailbox::CONFIG.path().clone())?
        .local_domain;

    let message_hash = eip191_hash(announcement_hash(
        domain_hash(local_domain, mailbox_addr.into(), VA_DOMAIN_KEY).to_vec(),
        &storage_location,
    ));

    // Recover pubkey from signature & verify.
    let pubkey = ctx.api.secp256k1_pubkey_recover(
        &message_hash,
        &signature[..64],
        // We subs 27 according to this - https://eips.ethereum.org/EIPS/eip-155
        signature[64] - 27,
        false,
    )?;
    let pk_hash = ctx.api.keccak256(&pubkey[1..]);
    let address = &pk_hash[12..];

    ensure!(address == validator.inner(), "pubkey mismatch");

    // Save validator.
    VALIDATORS.insert(ctx.storage, validator)?;

    // Append storage_locations.
    STORAGE_LOCATIONS.may_update(
        ctx.storage,
        validator,
        |maybe_storage_locations| -> StdResult<_> {
            let mut storage_locations = maybe_storage_locations.unwrap_or_default();
            storage_locations.insert(storage_location.clone());
            Ok(storage_locations)
        },
    )?;

    Ok(
        Response::new().add_event("validator-announcement", &EvtAnnouncement {
            sender: ctx.sender,
            validator,
            storage_location,
        })?,
    )
}

fn replay_hash(validator: HexByteArray<20>, storage_location: &str) -> Hash256 {
    [validator.inner(), storage_location.as_bytes()]
        .concat()
        .keccak256()
}

fn announcement_hash(mut domain_hash: Vec<u8>, storage_location: &str) -> Hash256 {
    let mut bz = vec![];
    bz.append(&mut domain_hash);
    bz.append(&mut storage_location.as_bytes().to_vec());

    bz.keccak256()
}

// ----------------------------------- tests -----------------------------------
