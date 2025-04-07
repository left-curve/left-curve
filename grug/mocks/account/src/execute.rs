use {
    crate::{Credential, InstantiateMsg, PUBLIC_KEY, PublicKey, SEQUENCE, SignDoc},
    anyhow::ensure,
    grug_types::{
        AuthCtx, AuthMode, AuthResponse, JsonDeExt, MutableCtx, Response, SignData, StdResult, Tx,
    },
};

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
    let sign_doc = SignDoc {
        sender: tx.sender,
        msgs: &tx.msgs,
        chain_id: &ctx.chain_id,
        sequence: credential.sequence,
    };
    let sign_data = sign_doc.to_sign_data()?;

    // Verify the signature.
    //
    // This is skipped when in simulation mode.
    //
    // Note the gas costs for verifying an Secp256k1 signature: 770,000 gas.
    // This cost are not accounted for in simulations.
    // It may be a good idea to manually add these to your simulation result.
    if let AuthMode::Check | AuthMode::Finalize = ctx.mode {
        ctx.api
            .secp256k1_verify(&sign_data, &credential.signature, &public_key)?;
    }

    // This account implementation doesn't make use of the transaction
    // backrunning feature, so we do not request a backrun.
    Ok(AuthResponse::new().request_backrun(false))
}
