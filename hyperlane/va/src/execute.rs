use {
    crate::{ANNOUNCE_FEE_PER_BYTE, MAILBOX, STORAGE_LOCATIONS},
    anyhow::ensure,
    grug::{HexByteArray, Inner, MutableCtx, Response, StdError, StorageQuerier, Uint128},
    hyperlane_types::{
        announcement_hash, domain_hash, eip191_hash, is_canonical_ecdsa_signature,
        va::{Announce, ExecuteMsg, Initialize, InstantiateMsg, VA_DOMAIN_KEY},
    },
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    MAILBOX.save(ctx.storage, &msg.mailbox)?;
    ANNOUNCE_FEE_PER_BYTE.save(ctx.storage, &msg.announce_fee_per_byte)?;

    Ok(Response::new().add_event(Initialize {
        creator: ctx.sender,
        mailbox: msg.mailbox,
        announce_fee_per_byte: msg.announce_fee_per_byte,
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
    // Calculate fee for announcement.
    let fee_per_byte = ANNOUNCE_FEE_PER_BYTE.load(ctx.storage)?;
    let fee = Uint128::new(fee_per_byte.amount.inner() * storage_location.len() as u128);
    let deposit = ctx.funds.into_one_coin_of_denom(&fee_per_byte.denom)?;

    ensure!(
        deposit.amount >= fee,
        "insufficient validator announce fee! required: {}, got: {}",
        fee,
        deposit.amount
    );

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
    // Ethereum uses recovery IDs 27, 28 instead
    // of 0, 1 (EIP-155 https://eips.ethereum.org/EIPS/eip-155).
    let v = signature[64];
    ensure!(v == 27 || v == 28, "invalid recovery id: {v}");
    ensure!(
        is_canonical_ecdsa_signature(signature.inner()),
        "non-canonical (high-s) signature"
    );

    let pubkey =
        ctx.api
            .secp256k1_pubkey_recover(&message_hash, &signature[..64], v - 27, false)?;
    let pk_hash = ctx.api.keccak256(&pubkey[1..]);
    let address = &pk_hash[12..];

    ensure!(address == validator.inner(), "pubkey mismatch");

    // Append storage_locations.
    STORAGE_LOCATIONS.may_update(ctx.storage, validator, |maybe| {
        let mut storage_locations = maybe.unwrap_or_default();
        storage_locations.try_push(storage_location.clone())?;

        Ok::<_, StdError>(storage_locations)
    })?;

    Ok(Response::new().add_event(Announce {
        sender: ctx.sender,
        validator,
        storage_location,
    })?)
}
