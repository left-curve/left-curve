use {
    anyhow::{anyhow, bail, ensure},
    base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine},
    dango_account_factory::{ACCOUNTS_BY_USER, KEYS, KEYS_BY_USER},
    dango_types::{
        auth::{ClientData, Credential, Key, Metadata, SignDoc},
        config::ACCOUNT_FACTORY_KEY,
    },
    grug::{
        Addr, AuthCtx, AuthMode, BorshDeExt, Counter, HashExt, JsonDeExt, JsonSerExt, Query, Tx,
    },
};

/// Expected sequence number of the next transaction this account sends.
///
/// All three account types (spot, margin, Safe) stores their sequences in this
/// same storage slot.
pub const NEXT_SEQUENCE: Counter<u32> = Counter::new("sequence", 0, 1);

/// Authenticate a transaction.
///
/// This logic is shared across all three account types.
pub fn authenticate_tx(
    ctx: AuthCtx,
    tx: Tx,
    // If the caller has already queried the factory address or deserialized the
    // metadata, they can be provided here, so we don't redo the work.
    maybe_factory: Option<Addr>,
    maybe_metadata: Option<Metadata>,
) -> anyhow::Result<()> {
    // Query the chain for account factory's address, if it's not already done.
    let factory = if let Some(factory) = maybe_factory {
        factory
    } else {
        ctx.querier.query_app_config(ACCOUNT_FACTORY_KEY)?
    };

    // Deserialize the transaction metadata, if it's not already done.
    let metadata = if let Some(metadata) = maybe_metadata {
        metadata
    } else {
        tx.data.deserialize_json()?
    };

    // Increment the sequence.
    let (sequence, _) = NEXT_SEQUENCE.increment(ctx.storage)?;

    // Query the account factory. We need to do three things:
    // - ensure the `tx.sender` is associated with the username;
    // - ensure the `key_hash` is associated wit the username;
    // - query the key by key hash.
    //
    // We use Wasm raw queries instead of smart queries to optimize on gas.
    // We also user the multi query to reduce the number of FFI calls.
    let key = {
        let [res1, res2, res3] = ctx.querier.query_multi([
            Query::wasm_raw(
                factory,
                ACCOUNTS_BY_USER.path((&metadata.username, tx.sender)),
            ),
            Query::wasm_raw(
                factory,
                KEYS_BY_USER.path((&metadata.username, metadata.key_hash)),
            ),
            Query::wasm_raw(factory, KEYS.path(metadata.key_hash)),
        ])?;

        // If the sender account is associated with the username, then an entry
        // must exist in the `ACCOUNTS_BY_USER` set, and the value should be
        // empty because we Borsh for encoding.
        ensure!(
            res1.as_wasm_raw().is_some_and(|bytes| bytes.is_empty()),
            "account {} isn't associated with user `{}`",
            tx.sender,
            metadata.username,
        );

        // Similarly, if the key hash is associated with the username, it must
        // be present in the `KEYS_BY_USER` set.
        ensure!(
            res2.as_wasm_raw().is_some_and(|bytes| bytes.is_empty()),
            "key hash {} isn't associated with user `{}`",
            metadata.key_hash,
            metadata.username
        );

        // Deserialize the key from Borsh bytes.
        res3.as_wasm_raw()
            .ok_or_else(|| anyhow!("key hash {} not found", metadata.key_hash))?
            .deserialize_borsh()?
    };

    // Compute the sign bytes.
    let sign_bytes = SignDoc {
        messages: tx.msgs,
        chain_id: ctx.chain_id,
        sequence: metadata.sequence,
    }
    .to_json_vec()?
    .hash256();

    // Verify sequence.
    match ctx.mode {
        // For `CheckTx`, we only make sure the tx's sequence is no smaller than
        // the stored sequence. This allows the account to broadcast multiple
        // txs for the same block.
        AuthMode::Check => {
            ensure!(
                metadata.sequence >= sequence,
                "sequence is too old: expecting at least {}, found {}",
                sequence,
                metadata.sequence
            );
        },
        // For `FinalizeBlock`, we make sure the tx's sequence matches exactly
        // the stored sequence.
        AuthMode::Finalize => {
            ensure!(
                metadata.sequence == sequence,
                "incorrect sequence: expecting {}, got {}",
                sequence,
                metadata.sequence
            );
        },
        // No need to verify sequence in simulation mode.
        AuthMode::Simulate => (),
    }

    // Verify signature.
    match ctx.mode {
        AuthMode::Check | AuthMode::Finalize => match (key, tx.credential.deserialize_json()?) {
            (Key::Secp256r1(pk), Credential::Passkey(cred)) => {
                // Generate the raw bytes that the Passkey should have signed.
                // See: <https://github.com/j0nl1/demo-passkey/blob/main/wasm/lib.rs#L59-L99>
                let signed_hash = {
                    let client_data = ClientData {
                        ty: "webauthn.get".to_string(),
                        challenge: URL_SAFE_NO_PAD.encode(sign_bytes),
                        origin: cred.origin,
                        cross_origin: cred.cross_origin,
                    };
                    let client_data_raw = client_data.to_json_vec()?;
                    let client_data_hash = ctx.api.sha2_256(&client_data_raw);

                    let signed_data = [
                        cred.authenticator_data.as_ref(),
                        client_data_hash.as_slice(),
                    ]
                    .concat();

                    // Note we use the FFI `sha2_256` method instead of `hash256`
                    // from `HashExt`, because we may change the hash function
                    // used in `HashExt` (we're exploring BLAKE3 over SHA-256).
                    // Passkey always signs over SHA-256 digests.
                    ctx.api.sha2_256(&signed_data)
                };

                ctx.api
                    .secp256r1_verify(&signed_hash, cred.sig.as_ref(), pk.as_ref())?;
            },
            (Key::Secp256k1(pk), Credential::Secp256k1(sig)) => {
                ctx.api
                    .secp256k1_verify(&sign_bytes, sig.as_ref(), pk.as_ref())?;
            },
            (Key::Ed25519(pk), Credential::Ed25519(sig)) => {
                ctx.api
                    .ed25519_verify(&sign_bytes, sig.as_ref(), pk.as_ref())?;
            },
            _ => bail!("key and credential types don't match!"),
        },
        // No need to verify signature in simulation mode.
        AuthMode::Simulate => (),
    }

    Ok(())
}
