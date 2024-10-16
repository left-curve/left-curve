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
                // Verify that Passkey has signed the correct data.
                // The data should be the SHA-256 hash of a `ClientData`, where
                // the challenge is the sign bytes.
                // See: <https://github.com/j0nl1/demo-passkey/blob/main/wasm/lib.rs#L59-L99>
                let signed_hash = {
                    let client_data: ClientData = cred.client_data.deserialize_json()?;
                    let sign_bytes_base64 = URL_SAFE_NO_PAD.encode(sign_bytes);

                    ensure!(
                        client_data.challenge == sign_bytes_base64,
                        "incorrect challenge: expecting {}, got {}",
                        sign_bytes_base64,
                        client_data.challenge
                    );

                    let client_data_hash = ctx.api.sha2_256(&cred.client_data);

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

                ctx.api.secp256r1_verify(&signed_hash, &cred.sig, &pk)?;
            },
            (Key::Secp256k1(pk), Credential::Secp256k1(sig)) => {
                ctx.api.secp256k1_verify(&sign_bytes, &sig, &pk)?;
            },
            (Key::Ed25519(pk), Credential::Ed25519(sig)) => {
                ctx.api.ed25519_verify(&sign_bytes, &sig, &pk)?;
            },
            _ => bail!("key and credential types don't match!"),
        },
        // No need to verify signature in simulation mode.
        AuthMode::Simulate => (),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_types::account_factory::Username,
        grug::{Addr, AuthMode, Hash160, MockContext, MockQuerier},
        std::str::FromStr,
    };

    /// Address of the account factory for use in the following tests.
    const ACCOUNT_FACTORY: Addr = Addr::mock(254);

    #[test]
    fn passkey_authentication() {
        let user_address = Addr::from_str("0x93841114860ba74d0a9fa88962268aff17365fc9").unwrap();
        let user_username = Username::from_str("test4").unwrap();
        let user_keyhash = Hash160::from_str("4466B77A86FB18EBA97080D56398B61470148059").unwrap();
        let user_key = Key::Secp256r1(
            [
                3, 32, 23, 59, 89, 52, 51, 126, 80, 201, 159, 243, 253, 222, 209, 56, 72, 217, 193,
                1, 195, 12, 83, 16, 188, 138, 208, 246, 53, 238, 156, 133, 163,
            ]
            .into(),
        );

        let tx = r#"{
            "sender": "0x93841114860ba74d0a9fa88962268aff17365fc9",
            "credential": {
              "passkey": {
                "sig": "BqtWfd8nTuTIiVipr/OcbeiBjsWmAp8e3VitWD+AekOmAPs/4dJkgjt7p+dB3ZJpqg6LHP+RX9bvALfgMoYh2Q==",
                "client_data": "eyJ0eXBlIjoid2ViYXV0aG4uZ2V0IiwiY2hhbGxlbmdlIjoiQmN3X1JrUDdDc3EtZWVFemw0ZWxFSWxTZXN0b055VVA1b21tUFJkU3VJQSIsIm9yaWdpbiI6Imh0dHA6Ly9sb2NhbGhvc3Q6NTA4MCIsImNyb3NzT3JpZ2luIjpmYWxzZX0=",
                "authenticator_data": "SZYN5YgOjGh0NBcPZHZgW4/krrmihjLHmVzzuoMdl2MZAAAAAA=="
              }
            },
            "data": {
              "key_hash": "4466B77A86FB18EBA97080D56398B61470148059",
              "username": "test4",
              "sequence": 0
            },
            "msgs": [
              {
                "transfer": {
                  "to": "0x123559ca94d734111f32cc7d603c3341c4d29a84",
                  "coins": {
                    "uusdc": "5"
                  }
                }
              }
            ],
            "gas_limit": 1116375
        }"#;

        let querier = MockQuerier::new()
            .with_app_config(ACCOUNT_FACTORY_KEY, ACCOUNT_FACTORY)
            .unwrap()
            .with_raw_contract_storage(ACCOUNT_FACTORY, |storage| {
                ACCOUNTS_BY_USER
                    .insert(storage, (&user_username, user_address))
                    .unwrap();
                KEYS_BY_USER
                    .insert(storage, (&user_username, user_keyhash))
                    .unwrap();
                KEYS.save(storage, user_keyhash, &user_key).unwrap()
            });

        let mut ctx = MockContext::new()
            .with_querier(querier)
            .with_chain_id("dev-2")
            .with_mode(AuthMode::Finalize);

        authenticate_tx(ctx.as_auth(), tx.deserialize_json().unwrap(), None, None).unwrap();
    }
}
