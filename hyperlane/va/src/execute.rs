use {
    crate::{MAILBOX, STORAGE_LOCATIONS},
    anyhow::ensure,
    grug::{HexByteArray, Inner, MutableCtx, Response, StdResult, StorageQuerier},
    hyperlane_types::{
        announcement_hash, domain_hash, eip191_hash,
        va::{Announce, ExecuteMsg, Initialize, InstantiateMsg, VA_DOMAIN_KEY},
    },
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    MAILBOX.save(ctx.storage, &msg.mailbox)?;

    Ok(Response::new().add_event(Initialize {
        creator: ctx.sender,
        mailbox: msg.mailbox,
    })?)
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
    // Make announcement digest.
    let mailbox = MAILBOX.load(ctx.storage)?;

    let local_domain = ctx
        .querier
        .query_wasm_path(mailbox, hyperlane_mailbox::CONFIG.path())?
        .local_domain;

    let message_hash = eip191_hash(announcement_hash(
        domain_hash(local_domain, mailbox.into(), VA_DOMAIN_KEY),
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

    Ok(Response::new().add_event(Announce {
        sender: ctx.sender,
        validator,
        storage_location,
    })?)
}
