use {
    alloy_dyn_abi::{Eip712Domain, TypedData},
    alloy_primitives::U160,
    anyhow::{anyhow, bail, ensure},
    base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine},
    dango_account_factory::{ACCOUNTS_BY_USER, KEYS, KEYS_BY_USER},
    dango_types::{
        auth::{ClientData, Credential, Key, Metadata, SignDoc},
        config::ACCOUNT_FACTORY_KEY,
    },
    grug::{
        json, Addr, AuthCtx, AuthMode, BorshDeExt, Counter, Inner, JsonDeExt, JsonSerExt, Query, Tx,
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
            (Key::Secp256k1(pk), Credential::Eip712(cred)) => {
                let TypedData {
                    resolver, domain, ..
                } = cred.typed_data.deserialize_json()?;

                // Recreate the EIP-712 data originally used for signing
                // Verify that the critical values in the transaction
                // such as the message and the verifying contract (sender)
                let typed_data = TypedData {
                    resolver,
                    domain: Eip712Domain {
                        name: domain.name,
                        verifying_contract: Some(
                            U160::from_be_bytes(ctx.contract.into_inner()).into(),
                        ),
                        ..Default::default()
                    },
                    primary_type: "Message".to_string(),
                    message: json!({
                        "chainId": ctx.chain_id,
                        "sequence": metadata.sequence,
                        "messages": tx.msgs,
                    }),
                };

                // EIP-712 hash used in the signature
                let sign_bytes = typed_data.eip712_signing_hash()?;

                ctx.api.secp256k1_verify(&sign_bytes.0, &cred.sig, &pk)?;
            },
            (key, credential) => {
                let sign_bytes = ctx.api.sha2_256(
                    &SignDoc {
                        messages: tx.msgs,
                        chain_id: ctx.chain_id,
                        sequence: metadata.sequence,
                    }
                    .to_json_vec()?,
                );
                match (key, credential) {
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
                    _ => bail!("key and credential types don't match!"),
                }
            },
        },
        // No need to verify signature in simulation mode.
        AuthMode::Simulate => (),
    }

    Ok(())
}

// ----------------------------------- tests -----------------------------------

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

    #[test]
    fn eip712_authentication() {
        let user_address = Addr::from_str("0x2e3d61d8cca8a774b884175fcf736e4c4e8060db").unwrap();
        let user_username = Username::from_str("test100").unwrap();
        let user_keyhash = Hash160::from_str("125DA0206939DD8D2DB125C8903F7F1EF96C6195").unwrap();
        let user_key = Key::Secp256k1(
            [
                2, 133, 171, 100, 31, 51, 234, 81, 118, 154, 124, 111, 48, 233, 1, 122, 227, 69,
                186, 224, 13, 210, 84, 190, 7, 38, 186, 1, 203, 80, 142, 47, 121,
            ]
            .into(),
        );

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
            .with_contract(Addr::from_str("0x2e3d61d8cca8a774b884175fcf736e4c4e8060db").unwrap())
            .with_chain_id("dev-2")
            .with_mode(AuthMode::Finalize);

        let tx = r#"{
          "sender": "0x2e3d61d8cca8a774b884175fcf736e4c4e8060db",
          "credential": {
            "eip712": {
              "sig": "BMbo/9aO/rPkhbt2hN1OaHl32QWME9BtLtwttbuUDyZmpnivpk73SLwDw6bc+3wsHSVxGVsARo5NtPPtQBDdvA==",
              "typed_data": "eyJ0eXBlcyI6eyJFSVA3MTJEb21haW4iOlt7Im5hbWUiOiJuYW1lIiwidHlwZSI6InN0cmluZyJ9LHsibmFtZSI6InZlcmlmeWluZ0NvbnRyYWN0IiwidHlwZSI6ImFkZHJlc3MifV0sIk1lc3NhZ2UiOlt7Im5hbWUiOiJjaGFpbklkIiwidHlwZSI6InN0cmluZyJ9LHsibmFtZSI6InNlcXVlbmNlIiwidHlwZSI6InVpbnQzMiJ9LHsibmFtZSI6Im1lc3NhZ2VzIiwidHlwZSI6IlR4TWVzc2FnZVtdIn1dLCJUeE1lc3NhZ2UiOlt7Im5hbWUiOiJ0cmFuc2ZlciIsInR5cGUiOiJUcmFuc2ZlciJ9XSwiVHJhbnNmZXIiOlt7Im5hbWUiOiJ0byIsInR5cGUiOiJhZGRyZXNzIn0seyJuYW1lIjoiY29pbnMiLCJ0eXBlIjoiQ29pbnMifV0sIkNvaW5zIjpbeyJuYW1lIjoidXVzZGMiLCJ0eXBlIjoic3RyaW5nIn1dfSwicHJpbWFyeVR5cGUiOiJNZXNzYWdlIiwiZG9tYWluIjp7Im5hbWUiOiJsb2NhbGhvc3QiLCJ2ZXJpZnlpbmdDb250cmFjdCI6IjB4MmUzZDYxZDhjY2E4YTc3NGI4ODQxNzVmY2Y3MzZlNGM0ZTgwNjBkYiJ9LCJtZXNzYWdlIjp7ImNoYWluSWQiOiJkZXYtMiIsIm1lc3NhZ2VzIjpbeyJ0cmFuc2ZlciI6eyJ0byI6IjB4MTIzNTU5Y2E5NGQ3MzQxMTFmMzJjYzdkNjAzYzMzNDFjNGQyOWE4NCIsImNvaW5zIjp7InV1c2RjIjoiMTAwMDAwMCJ9fX1dLCJzZXF1ZW5jZSI6MH19"
            }
          },
          "data": {
            "key_hash": "125DA0206939DD8D2DB125C8903F7F1EF96C6195",
            "username": "test100",
            "sequence": 0
          },
          "msgs": [
            {
              "transfer": {
                "to": "0x123559ca94d734111f32cc7d603c3341c4d29a84",
                "coins": {
                  "uusdc": "1000000"
                }
              }
            }
          ],
          "gas_limit": 1116931
        }"#;

        authenticate_tx(
            ctx.as_auth(),
            tx.deserialize_json::<Tx>().unwrap(),
            None,
            None,
        )
        .unwrap();
    }

    #[test]
    fn secp256k1_authentication() {
        let user_address = Addr::from_str("0x93841114860ba74d0a9fa88962268aff17365fc9").unwrap();
        let user_username = Username::from_str("owner").unwrap();
        let user_keyhash = Hash160::from_str("CB8E23B4BE5F8386E68AEDC900C9BFDA26519FDB").unwrap();
        let user_key = Key::Secp256k1(
            [
                2, 111, 13, 75, 148, 145, 161, 77, 233, 136, 36, 236, 240, 231, 244, 217, 82, 186,
                93, 163, 132, 66, 35, 3, 19, 17, 197, 180, 52, 200, 219, 178, 97,
            ]
            .into(),
        );

        let tx = r#"{
          "sender": "0x93841114860ba74d0a9fa88962268aff17365fc9",
          "credential": {
            "secp256k1": "GkkfQb81pEZJISbqAhJOZuZ5wgRyN0Q4HABD9HDAkxB516nV5d0UjRciNhh3RIg2Hh8nmS3jHFVl3NO34PVLNA=="
          },
          "data": {
            "key_hash": "CB8E23B4BE5F8386E68AEDC900C9BFDA26519FDB",
            "username": "owner",
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
          "gas_limit": 1046678
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
