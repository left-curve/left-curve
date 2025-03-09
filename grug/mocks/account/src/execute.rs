use {
    crate::{Credential, InstantiateMsg, PUBLIC_KEY, PublicKey, SEQUENCE},
    anyhow::ensure,
    grug_types::{
        Addr, AuthCtx, AuthMode, AuthResponse, JsonDeExt, JsonSerExt, Message, MutableCtx,
        Response, StdResult, Tx,
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
    sender: Addr,
    chain_id: &str,
    sequence: u32,
) -> StdResult<[u8; HASH_LEN]>
where
    Hasher: Fn(&[u8]) -> [u8; HASH_LEN],
{
    let mut prehash = Vec::new();
    // That there are multiple valid ways that the messages can be serialized
    // into JSON. Here we use `grug::to_json_vec` as the source of truth.
    prehash.extend(msgs.to_json_vec()?);
    prehash.extend(sender.as_ref());
    prehash.extend(chain_id.as_bytes());
    prehash.extend(sequence.to_be_bytes());
    Ok(hasher(&prehash))
}

pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> StdResult<Response> {
    // Save the public key in contract store
    PUBLIC_KEY.save(ctx.storage, &msg.public_key)?;

    Ok(Response::new())
}

pub fn update_key(ctx: MutableCtx, new_public_key: &PublicKey) -> anyhow::Result<Response> {
    // Only the account itself can update its key
    ensure!(ctx.sender == ctx.contract, "Nice try lol");

    // Save the new public key
    PUBLIC_KEY.save(ctx.storage, new_public_key)?;

    Ok(Response::new())
}

pub fn authenticate(ctx: AuthCtx, tx: Tx) -> anyhow::Result<AuthResponse> {
    let public_key = PUBLIC_KEY.load(ctx.storage)?;

    // Decode the credential, which should contain the sequence and signature.
    let credential: Credential = tx.credential.deserialize_json()?;

    // Incrementing the sequence. We expect the transaction to be signed by the
    // sequence _before_ the incrementing.
    let (sequence, _) = SEQUENCE.increment(ctx.storage)?;

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
        tx.sender,
        &ctx.chain_id,
        credential.sequence,
    )?;

    // Verify the signature.
    //
    // This is skipped when in simulation mode.
    //
    // Note the gas costs for verifying an Secp256k1 signature: 770,000 gas.
    // This cost are not accounted for in simulations.
    // It may be a good idea to manually add these to your simulation result.
    if let AuthMode::Check | AuthMode::Finalize = ctx.mode {
        ctx.api
            .secp256k1_verify(&hash, &credential.signature, &public_key)?;
    }

    // This account implementation doesn't make use of the transaction
    // backrunning feature, so we do not request a backrun.
    Ok(AuthResponse::new().request_backrun(false))
}
