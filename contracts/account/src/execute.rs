use {
    crate::{Credential, PublicKey, PUBLIC_KEY, SEQUENCE},
    anyhow::ensure,
    grug_types::{
        from_json_value, to_json_vec, Addr, AuthCtx, AuthMode, AuthResponse, Message, MutableCtx,
        Response, StdResult, Storage, Tx,
    },
};

/// Generate the bytes that the sender of a transaction needs to sign.
///
/// The bytes are defined as:
///
/// ```plain
/// bytes := hash(json(msgs) | sender | chain_id | sequence)
/// ```
///
/// Parameters:
///
/// - `hash` is a hash function; this account implementation uses SHA2-256;
/// - `msgs` is the list of messages in the transaction;
/// - `sender` is a 32 bytes address of the sender;
/// - `chain_id` is the chain ID in UTF-8 encoding;
/// - `sequence` is the sender account's sequence in 32-bit big endian encoding.
///
/// Chain ID and sequence are included in the sign bytes, as they are necessary
/// for preventing replat attacks (e.g. user signs a transaction for chain A;
/// attacker uses the signature to broadcast another transaction on chain B.)
pub fn make_sign_bytes<Hasher, const HASH_LEN: usize>(
    hasher: Hasher,
    msgs: &[Message],
    sender: &Addr,
    chain_id: &str,
    sequence: u32,
) -> StdResult<[u8; HASH_LEN]>
where
    Hasher: Fn(&[u8]) -> [u8; HASH_LEN],
{
    let mut prehash = Vec::new();
    // That there are multiple valid ways that the messages can be serialized
    // into JSON. Here we use `grug::to_json_vec` as the source of truth.
    prehash.extend(to_json_vec(&msgs)?);
    prehash.extend(sender.as_ref());
    prehash.extend(chain_id.as_bytes());
    prehash.extend(sequence.to_be_bytes());
    Ok(hasher(&prehash))
}

pub fn initialize(storage: &mut dyn Storage, public_key: &PublicKey) -> StdResult<Response> {
    // Save the public key in contract store
    PUBLIC_KEY.save(storage, public_key)?;

    // Initialize the sequence number to zero
    SEQUENCE.initialize(storage)?;

    Ok(Response::new())
}

pub fn update_key(ctx: MutableCtx, new_public_key: &PublicKey) -> anyhow::Result<Response> {
    // Only the account itself can update its key
    ensure!(ctx.sender == ctx.contract, "Nice try lol");

    // Save the new public key
    PUBLIC_KEY.save(ctx.storage, new_public_key)?;

    Ok(Response::new())
}

pub fn authenticate_tx(ctx: AuthCtx, tx: Tx) -> anyhow::Result<AuthResponse> {
    let public_key = PUBLIC_KEY.load(ctx.storage)?;
    let sequence = SEQUENCE.load(ctx.storage)?;
    let credential: Credential = from_json_value(tx.credential)?;

    match ctx.mode {
        // During `CheckTx`, ensure the tx's sequence is equal or greater than
        // the expected sequence.
        // This is to allow multiple transactions in the mempool from the same
        // account with different sequence numbers.
        AuthMode::Check => ensure!(
            credential.sequence >= sequence,
            "sequence is too old: expected at least {}, got {}",
            sequence,
            credential.sequence
        ),
        // During `FinalizeBlock`, ensure the tx's sequence equals exactly the
        // expected sequence.
        AuthMode::Finalize => ensure!(
            credential.sequence == sequence,
            "incorrect sequence number: expected {}, got {}",
            sequence,
            credential.sequence
        ),
        _ => (),
    };

    // Prepare the hash that is expected to have been signed.
    let hash = make_sign_bytes(
        // Note: We can't use a trait method as a function pointer. Need to use
        // a closure instead.
        |prehash| ctx.api.sha2_256(prehash),
        &tx.msgs,
        &tx.sender,
        &ctx.chain_id,
        credential.sequence,
    )?;

    // Verify the signature.
    //
    // This is skipped when in simulation mode.
    //
    // Note the gas costs for signature verification:
    // - Secp256r1: 1,880,000
    // - Secp256k1:   770,000
    // - Ethereum:  1,580,000
    //
    // These costs are not accounted for in simulations.
    // It may be a good idea to manually add these to your simulation result.
    if let AuthMode::Check | AuthMode::Finalize = ctx.mode {
        match &public_key {
            PublicKey::Secp256k1(bytes) => {
                ctx.api
                    .secp256k1_verify(&hash, &credential.signature, bytes)?;
            },
            PublicKey::Secp256r1(bytes) => {
                ctx.api
                    .secp256r1_verify(&hash, &credential.signature, bytes)?;
            },
        }
    }

    // Increment the sequence number
    SEQUENCE.increment(ctx.storage)?;

    Ok(AuthResponse::new_without_request_backrun(
        Response::new()
            .add_attribute("method", "before_tx")
            .add_attribute("sequence", sequence),
    ))
}
