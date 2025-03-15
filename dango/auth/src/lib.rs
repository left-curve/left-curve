use {
    alloy::{
        dyn_abi::{Eip712Domain, TypedData},
        primitives::U160,
    },
    anyhow::{bail, ensure},
    base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine},
    dango_account_factory::{ACCOUNTS_BY_USER, KEYS},
    dango_types::{
        auth::{
            ClientData, Credential, Key, Metadata, Nonce, SessionInfo, SignDoc, Signature,
            StandardCredential,
        },
        DangoQuerier,
    },
    grug::{
        json, Addr, Api, AuthCtx, AuthMode, Inner, Item, JsonDeExt, JsonSerExt, QuerierExt,
        StdResult, Storage, StorageQuerier, Tx,
    },
    std::collections::BTreeSet,
};

/// Max number of tracked nonces.
pub const MAX_SEEN_NONCES: usize = 20;

/// The most recent nonces that have been used to send transactions.
///
/// All three account types (spot, margin, multi) stores their nonces in this
/// same storage slot.
pub const SEEN_NONCES: Item<BTreeSet<Nonce>> = Item::new("seen_nonces");

/// Query the set of most recent nonce tracked.
pub fn query_seen_nonces(storage: &dyn Storage) -> StdResult<BTreeSet<Nonce>> {
    SEEN_NONCES
        .may_load(storage)
        .map(|opt| opt.unwrap_or_default())
}

/// Authenticate a transaction by ensuring:
///
/// - the username is associated with the sender account;
/// - the nonce is acceptible;
/// - the signature is authentic.
///
/// This logic is used by single-signature accounts (Spot and Margin).
pub fn authenticate_tx(
    ctx: AuthCtx,
    tx: Tx,
    // The deserialized metadata, if it's already done, so we don't have to redo
    // the work here.
    maybe_metadata: Option<Metadata>,
) -> anyhow::Result<()> {
    let factory = ctx.querier.query_account_factory()?;

    // Deserialize the transaction metadata, if it's not already done.
    let metadata = if let Some(metadata) = maybe_metadata {
        metadata
    } else {
        tx.data.clone().deserialize_json()?
    };

    // If the sender account is associated with the username, then an entry
    // must exist in the `ACCOUNTS_BY_USER` set, and the value should be
    // empty because we Borsh for encoding.
    ensure!(
        ctx.querier
            .query_wasm_raw(
                factory,
                ACCOUNTS_BY_USER.path((&metadata.username, tx.sender)),
            )?
            .is_some_and(|bytes| bytes.is_empty()),
        "account {} isn't associated with user `{}`",
        tx.sender,
        metadata.username,
    );

    verify_nonce_and_signature(ctx, tx, Some(factory), Some(metadata))
}

/// Ensure the nonce is acceptible and the signature is authentic.
///
/// Compared to [`authenticate_tx`](crate::authenticate_tx), this function skips
/// the part of verifying the username.
///
/// This is intended for the multi-signature accounts, where we ensure the
/// username is associated with the multisig _at the time a proposal was created_,
/// instead of _now_.
pub fn verify_nonce_and_signature(
    ctx: AuthCtx,
    tx: Tx,
    maybe_factory: Option<Addr>,
    maybe_metadata: Option<Metadata>,
) -> anyhow::Result<()> {
    // Query the chain for account factory's address, if it's not already done.
    let factory = if let Some(factory) = maybe_factory {
        factory
    } else {
        ctx.querier.query_account_factory()?
    };

    // Deserialize the transaction metadata, if it's not already done.
    let metadata = if let Some(metadata) = maybe_metadata {
        metadata
    } else {
        tx.data.deserialize_json()?
    };

    let sign_doc = SignDoc {
        gas_limit: tx.gas_limit,
        sender: ctx.contract,
        messages: tx.msgs,
        data: metadata.clone(),
    };

    match ctx.mode {
        AuthMode::Check | AuthMode::Finalize => {
            // Verify nonce.
            SEEN_NONCES.may_update(ctx.storage, |maybe_nonces| {
                let mut nonces = maybe_nonces.unwrap_or_default();

                match nonces.first() {
                    Some(&first) => {
                        // If there are nonces, we verify the nonce is not yet
                        // included as seen nonce and it is bigger than the
                        // oldest nonce.
                        ensure!(
                            !nonces.contains(&metadata.nonce),
                            "nonce is already seen: {}",
                            metadata.nonce
                        );

                        ensure!(
                            metadata.nonce > first,
                            "nonce is too old: {} < {}",
                            metadata.nonce,
                            first
                        );

                        // Remove the oldest nonce if max capacity is reached.
                        if nonces.len() == MAX_SEEN_NONCES {
                            nonces.pop_first();
                        }
                    },
                    None => {
                        // Ensure the first nonce is zero.
                        ensure!(metadata.nonce == 0, "first nonce must be 0");
                    },
                }

                nonces.insert(metadata.nonce);

                Ok(nonces)
            })?;

            // Verify tx expiration.
            if let Some(expiry) = metadata.expiry {
                ensure!(
                    expiry > ctx.block.timestamp,
                    "transaction expired at {:?}",
                    expiry
                );
            }

            let (
                StandardCredential {
                    key_hash,
                    signature,
                },
                session_credential,
            ) = match tx.credential.deserialize_json::<Credential>()? {
                Credential::Session(c) => (c.authorization.clone(), Some(c)),
                Credential::Standard(c) => (c, None),
            };

            // Query the key by key hash and username.
            let key = ctx
                .querier
                .query_wasm_path(factory, &KEYS.path((&metadata.username, key_hash)))?;

            if let Some(session) = session_credential {
                ensure!(
                    session.session_info.expire_at > ctx.block.timestamp,
                    "session expired at {:?}.",
                    session.session_info.expire_at
                );

                // Verify the `SessionInfo` signatures.
                verify_signature(
                    ctx.api,
                    key,
                    signature,
                    &VerifyData::Session(&session.session_info),
                )?;

                // Verify the `SignDoc` signature.
                verify_signature(
                    ctx.api,
                    Key::Secp256k1(session.session_info.session_key),
                    Signature::Secp256k1(session.session_signature),
                    &VerifyData::Standard {
                        chain_id: ctx.chain_id,
                        sign_doc,
                        nonce: metadata.nonce,
                    },
                )?;
            } else {
                // Verify the `SignDoc` signatures.
                verify_signature(ctx.api, key, signature, &VerifyData::Standard {
                    chain_id: ctx.chain_id,
                    sign_doc,
                    nonce: metadata.nonce,
                })?;
            }
        },
        // No need to verify nonce neither signature in simulation mode.
        AuthMode::Simulate => (),
    };

    Ok(())
}

fn verify_signature(
    api: &dyn Api,
    key: Key,
    signature: Signature,
    data: &VerifyData,
) -> anyhow::Result<()> {
    match (key, signature) {
        (Key::Secp256k1(pk), Signature::Eip712(cred)) => {
            let TypedData {
                resolver, domain, ..
            } = cred.typed_data.deserialize_json()?;

            // Recreate the EIP-712 data originally used for signing.
            // Verify that the critical values in the transaction such as
            // the message and the verifying contract (sender).
            let (verifying_contract, message) = match data {
                VerifyData::Standard {
                    sign_doc,
                    chain_id,
                    nonce,
                } => (
                    Some(U160::from_be_bytes(sign_doc.sender.into_inner()).into()),
                    json!({
                        "gas_limit": sign_doc.gas_limit,
                        "metadata": {
                            "username": sign_doc.data.username,
                            "chain_id": chain_id,
                            "nonce": nonce,
                            "expiry": sign_doc.data.expiry,
                        },
                        "messages": sign_doc.messages,
                    }),
                ),
                VerifyData::Session(session_info) => (
                    None,
                    json!({
                        "session_key": session_info.session_key,
                        "expire_at": session_info.expire_at,
                    }),
                ),
            };

            // EIP-712 hash used in the signature.
            let sign_bytes = TypedData {
                resolver,
                domain: Eip712Domain {
                    name: domain.name,
                    verifying_contract,
                    ..Default::default()
                },
                primary_type: "Message".to_string(),
                message: message.into_inner(),
            }
            .eip712_signing_hash()?;

            api.secp256k1_verify(&sign_bytes.0, &cred.sig, &pk)?;
        },
        (Key::Secp256r1(pk), Signature::Passkey(cred)) => {
            let signed_hash = {
                let client_data: ClientData = cred.client_data.deserialize_json()?;

                let sign_bytes = api.sha2_256(&data.as_sign_bytes()?);
                let sign_bytes_base64 = URL_SAFE_NO_PAD.encode(sign_bytes);

                ensure!(
                    client_data.challenge == sign_bytes_base64,
                    "incorrect challenge: expecting {}, got {}",
                    sign_bytes_base64,
                    client_data.challenge
                );

                let client_data_hash = api.sha2_256(&cred.client_data);

                let signed_data = [
                    cred.authenticator_data.as_ref(),
                    client_data_hash.as_slice(),
                ]
                .concat();

                // Note we use the FFI `sha2_256` method instead of `hash256`
                // from `HashExt`, because we may change the hash function
                // used in `HashExt` (we're exploring BLAKE3 over SHA-256).
                // Passkey always signs over SHA-256 digests.
                api.sha2_256(&signed_data)
            };

            api.secp256r1_verify(&signed_hash, &cred.sig, &pk)?;
        },
        (Key::Secp256k1(pk), Signature::Secp256k1(sig)) => {
            let sign_bytes = api.sha2_256(&data.as_sign_bytes()?);

            api.secp256k1_verify(&sign_bytes, &sig, &pk)?;
        },
        _ => bail!("key and credential types don't match!"),
    }
    Ok(())
}

enum VerifyData<'a> {
    Session(&'a SessionInfo),
    Standard {
        sign_doc: SignDoc,
        chain_id: String,
        nonce: Nonce,
    },
}

impl VerifyData<'_> {
    fn as_sign_bytes(&self) -> StdResult<Vec<u8>> {
        match self {
            VerifyData::Session(session_info) => session_info.to_json_vec(),
            VerifyData::Standard { sign_doc, .. } => sign_doc.to_json_vec(),
        }
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_types::{
            account_factory::Username,
            config::{AppAddresses, AppConfig},
        },
        grug::{btree_map, Addr, AuthMode, Hash256, MockContext, MockQuerier},
        std::str::FromStr,
    };

    /// Address of the account factory for use in the following tests.
    const ACCOUNT_FACTORY: Addr = Addr::mock(254);

    #[test]
    fn passkey_authentication() {
        let user_address = Addr::from_str("0x4857ff85aa9d69c73bc86eb45949455b45cca580").unwrap();
        let user_username = Username::from_str("passkey").unwrap();
        let user_keyhash =
            Hash256::from_str("F060857303FA03DA27F41C8EBEA9A7E891CE05E840321CB302EE515E84B9D82B")
                .unwrap();
        let user_key = Key::Secp256r1(
            [
                3, 199, 78, 155, 236, 166, 144, 61, 14, 162, 252, 123, 39, 173, 138, 43, 78, 85,
                27, 52, 251, 242, 61, 201, 115, 217, 122, 234, 164, 24, 51, 48, 190,
            ]
            .into(),
        );

        let tx = r#"{
          "sender": "0x4857ff85aa9d69c73bc86eb45949455b45cca580",
          "credential": {
            "standard": {
              "signature": {
                "passkey": {
                  "sig": "NmW5+jQ5lGlj4FyisBq6kA6sQ4gW2usbthLE6kl8sQPHXQVNdoHk+ZjP4YHg0p7Fl6Z+O79tHqPnm7vX1IkOsA==",
                  "client_data": "eyJ0eXBlIjoid2ViYXV0aG4uZ2V0IiwiY2hhbGxlbmdlIjoiWXd0dkNHdld3Q052UnhhNUxtdFE0OTZ4WV9lM1NtVkMwUnJ2SGF1TktuZyIsIm9yaWdpbiI6Imh0dHA6Ly9sb2NhbGhvc3Q6NTA4MCIsImNyb3NzT3JpZ2luIjpmYWxzZX0=",
                  "authenticator_data": "SZYN5YgOjGh0NBcPZHZgW4/krrmihjLHmVzzuoMdl2MZAAAAAA=="
                }
              },
              "key_hash": "F060857303FA03DA27F41C8EBEA9A7E891CE05E840321CB302EE515E84B9D82B"
            }
          },
          "data": {
            "username": "passkey",
            "nonce": 0,
            "chain_id": "dev-3"
          },
          "msgs": [
            {
              "transfer": {
                "0x064c5e20b422b5d817fe800119dac0ab43b17a80": {
                  "uusdc": "1000000"
                }
              }
            }
          ],
          "gas_limit": 2566613
        }"#;

        let querier = MockQuerier::new()
            .with_app_config(AppConfig {
                addresses: AppAddresses {
                    account_factory: ACCOUNT_FACTORY,
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap()
            .with_raw_contract_storage(ACCOUNT_FACTORY, |storage| {
                ACCOUNTS_BY_USER
                    .insert(storage, (&user_username, user_address))
                    .unwrap();
                KEYS.save(storage, (&user_username, user_keyhash), &user_key)
                    .unwrap();
            });

        let mut ctx = MockContext::new()
            .with_querier(querier)
            .with_contract(user_address)
            .with_chain_id("dev-3")
            .with_mode(AuthMode::Finalize);

        authenticate_tx(ctx.as_auth(), tx.deserialize_json().unwrap(), None).unwrap();
    }

    #[test]
    fn eip712_authentication() {
        let user_address = Addr::from_str("0x227e7e3d56ffd984ba6e3ead892f5676fa722a16").unwrap();
        let user_username = Username::from_str("javier_1").unwrap();
        let user_keyhash =
            Hash256::from_str("622FE2E6EDABB23602D87CC65E4FE2749A232B32035651C99591A098AAD8629B")
                .unwrap();
        let user_key = Key::Secp256k1(
            [
                3, 115, 37, 57, 128, 37, 222, 189, 9, 42, 142, 196, 85, 27, 226, 112, 136, 195,
                174, 6, 40, 39, 221, 182, 179, 146, 169, 207, 108, 218, 67, 27, 71,
            ]
            .into(),
        );

        let querier = MockQuerier::new()
            .with_app_config(AppConfig {
                addresses: AppAddresses {
                    account_factory: ACCOUNT_FACTORY,
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap()
            .with_raw_contract_storage(ACCOUNT_FACTORY, |storage| {
                ACCOUNTS_BY_USER
                    .insert(storage, (&user_username, user_address))
                    .unwrap();
                KEYS.save(storage, (&user_username, user_keyhash), &user_key)
                    .unwrap();
            });

        let mut ctx = MockContext::new()
            .with_querier(querier)
            .with_contract(user_address)
            .with_chain_id("dev-3")
            .with_mode(AuthMode::Finalize);

        let tx = r#"{
          "sender": "0x227e7e3d56ffd984ba6e3ead892f5676fa722a16",
          "credential": {
            "standard": {
              "signature": {
                "eip712": {
                  "sig": "ygSIjqH++C55ksLXKF/9hPxBpcLECtGlsX/fxB5eTqNbXV9CUjp1PtBBu/36SYxrkY4SVYvqrsZFxcr2CBzKFQ==",
                  "typed_data": "eyJ0eXBlcyI6eyJFSVA3MTJEb21haW4iOlt7Im5hbWUiOiJuYW1lIiwidHlwZSI6InN0cmluZyJ9LHsibmFtZSI6InZlcmlmeWluZ0NvbnRyYWN0IiwidHlwZSI6ImFkZHJlc3MifV0sIk1lc3NhZ2UiOlt7Im5hbWUiOiJtZXRhZGF0YSIsInR5cGUiOiJNZXRhZGF0YSJ9LHsibmFtZSI6Imdhc19saW1pdCIsInR5cGUiOiJ1aW50MzIifSx7Im5hbWUiOiJtZXNzYWdlcyIsInR5cGUiOiJUeE1lc3NhZ2VbXSJ9XSwiTWV0YWRhdGEiOlt7Im5hbWUiOiJ1c2VybmFtZSIsInR5cGUiOiJzdHJpbmcifSx7Im5hbWUiOiJjaGFpbl9pZCIsInR5cGUiOiJzdHJpbmcifSx7Im5hbWUiOiJub25jZSIsInR5cGUiOiJ1aW50MzIifV0sIlR4TWVzc2FnZSI6W3sibmFtZSI6InRyYW5zZmVyIiwidHlwZSI6IlRyYW5zZmVyIn1dLCJUcmFuc2ZlciI6W3sibmFtZSI6InRvIiwidHlwZSI6ImFkZHJlc3MifSx7Im5hbWUiOiJjb2lucyIsInR5cGUiOiJDb2lucyJ9XSwiQ29pbnMiOlt7Im5hbWUiOiJ1dXNkYyIsInR5cGUiOiJzdHJpbmcifV19LCJwcmltYXJ5VHlwZSI6Ik1lc3NhZ2UiLCJkb21haW4iOnsibmFtZSI6ImxvY2FsaG9zdCIsInZlcmlmeWluZ0NvbnRyYWN0IjoiMHgyMjdlN2UzZDU2ZmZkOTg0YmE2ZTNlYWQ4OTJmNTY3NmZhNzIyYTE2In0sIm1lc3NhZ2UiOnsibWV0YWRhdGEiOnsidXNlcm5hbWUiOiJqYXZpZXJfMSIsIm5vbmNlIjowLCJjaGFpbl9pZCI6ImRldi0zIn0sImdhc19saW1pdCI6MjU2NjU1OCwibWVzc2FnZXMiOlt7InRyYW5zZmVyIjp7InRvIjoiMHgwNjRjNWUyMGI0MjJiNWQ4MTdmZTgwMDExOWRhYzBhYjQzYjE3YTgwIiwiY29pbnMiOnsidXVzZGMiOiIxMDAwMDAwIn19fV19fQ=="
                }
              },
              "key_hash": "622FE2E6EDABB23602D87CC65E4FE2749A232B32035651C99591A098AAD8629B"
            }
          },
          "data": {
            "username": "javier_1",
            "nonce": 0,
            "chain_id": "dev-3"
          },
          "msgs": [
            {
              "transfer": {
                "0x064c5e20b422b5d817fe800119dac0ab43b17a80": {
                  "uusdc": "1000000"
                }
              }
            }
          ],
          "gas_limit": 2566558
        }"#;

        authenticate_tx(ctx.as_auth(), tx.deserialize_json::<Tx>().unwrap(), None).unwrap();
    }

    #[test]
    fn secp256k1_authentication() {
        let user_address = Addr::from_str("0xb86b2d96971c32f68241df04691479edb6a9cd3b").unwrap();
        let user_username = Username::from_str("owner").unwrap();
        let user_keyhash =
            Hash256::from_str("06E54A648823A1F12E1F03FED193C9FE0C030A65507FF09066BF9E067CD375D2")
                .unwrap();
        let user_key = Key::Secp256k1(
            [
                2, 120, 247, 183, 217, 61, 169, 181, 166, 46, 40, 67, 65, 132, 209, 195, 55, 194,
                194, 141, 76, 237, 41, 23, 147, 33, 90, 182, 238, 137, 215, 255, 248,
            ]
            .into(),
        );

        let tx = r#"{
          "sender": "0xb86b2d96971c32f68241df04691479edb6a9cd3b",
          "credential": {
            "standard": {
              "signature": {
                "secp256k1": "YDh0d3Fu38vVRarTXssImRkORCDiyKkVYj22h8mOSAtohM3alOJkO+q/PLSo/+7WlFytT3CKJp04mSluCW0dOQ=="
              },
              "key_hash": "06E54A648823A1F12E1F03FED193C9FE0C030A65507FF09066BF9E067CD375D2"
            }
          },
          "data": {
            "username": "owner",
            "nonce": 0,
            "chain_id": "dev-3"
        },
          "msgs": [
            {
              "transfer": {
                "0x064c5e20b422b5d817fe800119dac0ab43b17a80": {
                  "uusdc": "10000"
                }
              }
            }
          ],
          "gas_limit": 2566278
        }"#;

        let querier = MockQuerier::new()
            .with_app_config(AppConfig {
                addresses: AppAddresses {
                    account_factory: ACCOUNT_FACTORY,
                    // Address below don't matter for this test.
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap()
            .with_raw_contract_storage(ACCOUNT_FACTORY, |storage| {
                ACCOUNTS_BY_USER
                    .insert(storage, (&user_username, user_address))
                    .unwrap();
                KEYS.save(storage, (&user_username, user_keyhash), &user_key)
                    .unwrap();
            });

        let mut ctx = MockContext::new()
            .with_querier(querier)
            .with_contract(user_address)
            .with_chain_id("dev-3")
            .with_mode(AuthMode::Finalize);

        authenticate_tx(ctx.as_auth(), tx.deserialize_json().unwrap(), None).unwrap();
    }

    #[test]
    fn session_key_authentication() {
        let user_address = Addr::from_str("0x1128323d3502087eab68007e0717ccf36d9e96fd").unwrap();
        let user_username = Username::from_str("javier_1").unwrap();
        let user_keyhash =
            Hash256::from_str("622FE2E6EDABB23602D87CC65E4FE2749A232B32035651C99591A098AAD8629B")
                .unwrap();
        let user_key = Key::Secp256k1(
            [
                3, 115, 37, 57, 128, 37, 222, 189, 9, 42, 142, 196, 85, 27, 226, 112, 136, 195,
                174, 6, 40, 39, 221, 182, 179, 146, 169, 207, 108, 218, 67, 27, 71,
            ]
            .into(),
        );

        let querier = MockQuerier::new()
            .with_app_config(AppConfig {
                addresses: AppAddresses {
                    account_factory: ACCOUNT_FACTORY,
                    ..Default::default()
                },
                collateral_powers: btree_map! {},
                ..Default::default()
            })
            .unwrap()
            .with_raw_contract_storage(ACCOUNT_FACTORY, |storage| {
                ACCOUNTS_BY_USER
                    .insert(storage, (&user_username, user_address))
                    .unwrap();
                KEYS.save(storage, (&user_username, user_keyhash), &user_key)
                    .unwrap();
            });

        let mut ctx = MockContext::new()
            .with_querier(querier)
            .with_contract(user_address)
            .with_chain_id("dev-3")
            .with_mode(AuthMode::Finalize);

        let tx = r#"{
          "sender": "0x1128323d3502087eab68007e0717ccf36d9e96fd",
          "credential": {
            "session": {
              "session_info": {
                "session_key": "A2W3zyOByPqqPDeX2iGVX3S+/Kg3dDxuQPPASRdRsxIR",
                "expire_at": "149886405843120000000"
              },
              "authorization": {
                "key_hash": "622FE2E6EDABB23602D87CC65E4FE2749A232B32035651C99591A098AAD8629B",
                "signature": {
                  "eip712": {
                    "sig": "Iv/yinJ7jCpT9dYi5bVmz0GDsXjkPA6h8+jnbkYGSFBTEwShLBpHrpONM2qP9ZcolY/5jxhpcqHZEamfelf2yQ==",
                    "typed_data": "eyJkb21haW4iOnsibmFtZSI6IkRhbmdvQXJiaXRyYXJ5TWVzc2FnZSJ9LCJtZXNzYWdlIjp7InNlc3Npb25fa2V5IjoiQTJXM3p5T0J5UHFxUERlWDJpR1ZYM1MrL0tnM2REeHVRUFBBU1JkUnN4SVIiLCJleHBpcmVfYXQiOiIxNDk4ODY0MDU4NDMxMjAwMDAwMDAifSwicHJpbWFyeVR5cGUiOiJNZXNzYWdlIiwidHlwZXMiOnsiRUlQNzEyRG9tYWluIjpbeyJuYW1lIjoibmFtZSIsInR5cGUiOiJzdHJpbmcifV0sIk1lc3NhZ2UiOlt7Im5hbWUiOiJzZXNzaW9uX2tleSIsInR5cGUiOiJzdHJpbmcifSx7Im5hbWUiOiJleHBpcmVfYXQiLCJ0eXBlIjoic3RyaW5nIn1dfX0="
                  }
                }
              },
              "session_signature": "yQQ45KtHDGo8itCmY59MBo9JPfA2/A+vEvNFiFnvLM1kilmxmGe0oFpeCSYlwS5uDxa7AZNp+620BlJ6dA0XcQ=="
            }
          },
          "data": {
            "username": "javier_1",
            "nonce": 0,
            "chain_id": "dev-3"
          },
          "msgs": [
            {
              "transfer": {
                "to": "0x064c5e20b422b5d817fe800119dac0ab43b17a80",
                "coins": {
                  "uusdc": "1000000"
                }
              }
            }
          ],
          "gas_limit": 2566260
        }"#;

        authenticate_tx(ctx.as_auth(), tx.deserialize_json::<Tx>().unwrap(), None).unwrap();
    }

    #[test]
    fn tracked_nonces_works() {}
}
