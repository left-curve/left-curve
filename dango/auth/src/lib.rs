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

    // Ensure the chain ID in metadata matches the context.
    ensure!(
        metadata.chain_id == ctx.chain_id,
        "chain ID mismatch: expecting `{}`, got `{}`",
        ctx.chain_id,
        metadata.chain_id
    );

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
<<<<<<< Updated upstream
            VerifyData::Session(session_info) => session_info.to_json_value()?.to_json_vec(),
            VerifyData::Standard { sign_doc, .. } => sign_doc.to_json_value()?.to_json_vec(),
=======
            VerifyData::Session(session_info) => session_info.to_json_value()?,
            VerifyData::Standard { sign_doc, .. } => sign_doc.to_json_value()?,
>>>>>>> Stashed changes
        }
        .to_json_vec()
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
        grug::{btree_map, Addr, AuthMode, Hash256, MockContext, MockQuerier, ResultExt},
        std::str::FromStr,
    };

    /// Address of the account factory for use in the following tests.
    const ACCOUNT_FACTORY: Addr = Addr::mock(254);

    #[test]
    fn passkey_authentication() {
        let user_address = Addr::from_str("0x94e4e04fbf35a0e67c559fe1c9579de9fdd0f6ed").unwrap();
        let user_username = Username::from_str("pass_local").unwrap();
        let user_keyhash =
            Hash256::from_str("8E60264C2887C814C0C1E873A66F51F294149EFC3161CB1A195277D330927F31")
                .unwrap();
        let user_key = Key::Secp256r1(
            [
                2, 244, 56, 241, 68, 190, 202, 32, 187, 114, 180, 9, 199, 217, 8, 121, 69, 155,
                181, 78, 55, 162, 133, 63, 56, 242, 30, 111, 63, 93, 80, 217, 53,
            ]
            .into(),
        );

        let tx = r#"{
            "credential":{
                "standard":{
                "key_hash":"8E60264C2887C814C0C1E873A66F51F294149EFC3161CB1A195277D330927F31",
                "signature":{
                    "passkey":{
                    "authenticator_data":"SZYN5YgOjGh0NBcPZHZgW4/krrmihjLHmVzzuoMdl2MZAAAAAA==",
                    "client_data":"eyJ0eXBlIjoid2ViYXV0aG4uZ2V0IiwiY2hhbGxlbmdlIjoiS1I3OXRVWHp4R2liTzloVmhQNlk0TmZGcmRsOHg0dVR4cm9RbU5HTGhzayIsIm9yaWdpbiI6Imh0dHA6Ly9sb2NhbGhvc3Q6NTA4MCIsImNyb3NzT3JpZ2luIjpmYWxzZSwib3RoZXJfa2V5c19jYW5fYmVfYWRkZWRfaGVyZSI6ImRvIG5vdCBjb21wYXJlIGNsaWVudERhdGFKU09OIGFnYWluc3QgYSB0ZW1wbGF0ZS4gU2VlIGh0dHBzOi8vZ29vLmdsL3lhYlBleCJ9",
                    "sig":"fTKkzapyn0e3Q27ARsdxTGDQA0rSv/hmvSp++xJdKk4yBgP4CxqidByWOA0FmVQ2wBuob9BINpu7Eho+UFFroQ=="
                    }
                }
                }
            },
            "data":{
                "chain_id":"dev-5",
                "nonce":0,
                "username":"pass_local"
            },
            "gas_limit":2448139,
            "msgs":[
                {
                "transfer":{
                    "0x33361de42571d6aa20c37daa6da4b5ab67bfaad9":{
                    "hyp/eth/usdc":"1000000"
                    }
                }
                }
            ],
            "sender":"0x94e4e04fbf35a0e67c559fe1c9579de9fdd0f6ed"
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
            .with_chain_id("dev-5")
            .with_mode(AuthMode::Finalize);

        authenticate_tx(ctx.as_auth(), tx.deserialize_json().unwrap(), None).should_succeed();
    }

    #[test]
    fn eip712_authentication() {
        let user_address = Addr::from_str("0xb66227cf4ea800b6b19aed198395fd0a2d80ee1d").unwrap();
        let user_username = Username::from_str("javier").unwrap();
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
            .with_chain_id("dev-6")
            .with_mode(AuthMode::Finalize);

        let tx = r#"{
            "credential":{
                "standard":{
                "key_hash":"622FE2E6EDABB23602D87CC65E4FE2749A232B32035651C99591A098AAD8629B",
                "signature":{
                    "eip712":{
                    "sig":"qhvraAO/AnyJO621ig38y8Rc4k12UtuKurqTjMOCHogR5UpyptvZ2lTl0gH0wjFiUTNp3uFe+WAh760HWq2Mnw==",
                    "typed_data":"eyJ0eXBlcyI6eyJFSVA3MTJEb21haW4iOlt7Im5hbWUiOiJuYW1lIiwidHlwZSI6InN0cmluZyJ9LHsibmFtZSI6InZlcmlmeWluZ0NvbnRyYWN0IiwidHlwZSI6ImFkZHJlc3MifV0sIk1lc3NhZ2UiOlt7Im5hbWUiOiJtZXRhZGF0YSIsInR5cGUiOiJNZXRhZGF0YSJ9LHsibmFtZSI6Imdhc19saW1pdCIsInR5cGUiOiJ1aW50MzIifSx7Im5hbWUiOiJtZXNzYWdlcyIsInR5cGUiOiJUeE1lc3NhZ2VbXSJ9XSwiTWV0YWRhdGEiOlt7Im5hbWUiOiJ1c2VybmFtZSIsInR5cGUiOiJzdHJpbmcifSx7Im5hbWUiOiJjaGFpbl9pZCIsInR5cGUiOiJzdHJpbmcifSx7Im5hbWUiOiJub25jZSIsInR5cGUiOiJ1aW50MzIifV0sIlR4TWVzc2FnZSI6W3sibmFtZSI6InRyYW5zZmVyIiwidHlwZSI6IlRyYW5zZmVyIn1dLCJUcmFuc2ZlciI6W3sibmFtZSI6IjB4MzMzNjFkZTQyNTcxZDZhYTIwYzM3ZGFhNmRhNGI1YWI2N2JmYWFkOSIsInR5cGUiOiJDb2luMCJ9XSwiQ29pbjAiOlt7Im5hbWUiOiJoeXAvZXRoL3VzZGMiLCJ0eXBlIjoic3RyaW5nIn1dfSwicHJpbWFyeVR5cGUiOiJNZXNzYWdlIiwiZG9tYWluIjp7Im5hbWUiOiJkYW5nbyIsInZlcmlmeWluZ0NvbnRyYWN0IjoiMHhiNjYyMjdjZjRlYTgwMGI2YjE5YWVkMTk4Mzk1ZmQwYTJkODBlZTFkIn0sIm1lc3NhZ2UiOnsibWV0YWRhdGEiOnsiY2hhaW5faWQiOiJkZXYtNiIsInVzZXJuYW1lIjoiamF2aWVyIiwibm9uY2UiOjB9LCJnYXNfbGltaXQiOjI0NDgxMzksIm1lc3NhZ2VzIjpbeyJ0cmFuc2ZlciI6eyIweDMzMzYxZGU0MjU3MWQ2YWEyMGMzN2RhYTZkYTRiNWFiNjdiZmFhZDkiOnsiaHlwL2V0aC91c2RjIjoiMTAwMDAwMCJ9fX1dfX0="
                    }
                }
                }
            },
            "data":{
                "chain_id":"dev-6",
                "nonce":0,
                "username":"javier"
            },
            "gas_limit":2448139,
            "msgs":[
                {
                "transfer":{
                    "0x33361de42571d6aa20c37daa6da4b5ab67bfaad9":{
                    "hyp/eth/usdc":"1000000"
                    }
                }
                }
            ],
            "sender":"0xb66227cf4ea800b6b19aed198395fd0a2d80ee1d"
        }
        "#;

        authenticate_tx(ctx.as_auth(), tx.deserialize_json::<Tx>().unwrap(), None).should_succeed();
    }

    #[test]
    fn secp256k1_authentication() {
        let user_address = Addr::from_str("0x33361de42571d6aa20c37daa6da4b5ab67bfaad9").unwrap();
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
        "credential":{
            "standard":{
            "key_hash":"06E54A648823A1F12E1F03FED193C9FE0C030A65507FF09066BF9E067CD375D2",
            "signature":{
                "secp256k1":"CLlermDLySBkXKiU33LTPtzeOt8Rp0W7bKs3nMdRbEZUDumK7fldZ6WTxCjvg7apTPO1dxqFUzsbvwfQFaxG+w=="
            }
            }
        },
        "data":{
            "chain_id":"dev-6",
            "nonce":0,
            "username":"owner"
        },
        "gas_limit":2448142,
        "msgs":[
            {
            "transfer":{
                "0xb66227cf4ea800b6b19aed198395fd0a2d80ee1d":{
                "hyp/eth/usdc":"100"
                }
            }
            }
        ],
        "sender":"0x33361de42571d6aa20c37daa6da4b5ab67bfaad9"
        }
        "#;

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

        // With the incorrect chain ID. Should fail.
        let mut ctx = MockContext::new()
            .with_querier(querier)
            .with_contract(user_address)
            .with_chain_id("not-dev-6")
            .with_mode(AuthMode::Finalize);

        authenticate_tx(ctx.as_auth(), tx.deserialize_json().unwrap(), None)
            .should_fail_with_error("chain ID mismatch");

        // With the correct chain ID.
        let mut ctx = MockContext::new()
            .with_querier(ctx.querier)
            .with_contract(user_address)
            .with_chain_id("dev-6")
            .with_mode(AuthMode::Finalize);

        authenticate_tx(ctx.as_auth(), tx.deserialize_json().unwrap(), None).should_succeed();
    }

    #[test]
    fn session_key_authentication() {
        let user_address = Addr::from_str("0xb66227cf4ea800b6b19aed198395fd0a2d80ee1d").unwrap();
        let user_username = Username::from_str("javier").unwrap();
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
            .with_chain_id("dev-6")
            .with_mode(AuthMode::Finalize);

        let tx = r#"{
            "credential":{
                "session":{
                "authorization":{
                    "key_hash":"622FE2E6EDABB23602D87CC65E4FE2749A232B32035651C99591A098AAD8629B",
                    "signature":{
                    "eip712":{
                        "sig":"t0QXAD4vGR9p5bvVG3CUvrM4dgPAUBJZkiJqvYxRfgckJlcaPf4NELBiXRfg3ZVvpkDraWh3OyG02FpAzec/PQ==",
                        "typed_data":"eyJkb21haW4iOnsibmFtZSI6IkRhbmdvQXJiaXRyYXJ5TWVzc2FnZSJ9LCJtZXNzYWdlIjp7InNlc3Npb25fa2V5IjoiQXNnRkd4NmJDenh6UElMQUo5Vkx5VDNFb3V5VHlsekNqcFJ2c0dleVVXOHQiLCJleHBpcmVfYXQiOiIxNzQzMDE1MzI3MTg3In0sInByaW1hcnlUeXBlIjoiTWVzc2FnZSIsInR5cGVzIjp7IkVJUDcxMkRvbWFpbiI6W3sibmFtZSI6Im5hbWUiLCJ0eXBlIjoic3RyaW5nIn1dLCJNZXNzYWdlIjpbeyJuYW1lIjoic2Vzc2lvbl9rZXkiLCJ0eXBlIjoic3RyaW5nIn0seyJuYW1lIjoiZXhwaXJlX2F0IiwidHlwZSI6InN0cmluZyJ9XX19"
                    }
                    }
                },
                "session_info":{
                    "expire_at":"1743015327187",
                    "session_key":"AsgFGx6bCzxzPILAJ9VLyT3EouyTylzCjpRvsGeyUW8t"
                },
                "session_signature":"8nK6wFA2AkafMOiTt7cfQfV7yBCZa477sxguieK2sI0C1iCOdX4OfiMAZYIOadZjG/+4m6IkvXe/GgTNloP72A=="
                }
            },
            "data":{
                "chain_id":"dev-6",
                "nonce":0,
                "username":"javier"
            },
            "gas_limit":2448139,
            "msgs":[
                {
                "transfer":{
                    "0x33361de42571d6aa20c37daa6da4b5ab67bfaad9":{
                    "hyp/eth/usdc":"1000000"
                    }
                }
                }
            ],
            "sender":"0xb66227cf4ea800b6b19aed198395fd0a2d80ee1d"
        }"#;

        authenticate_tx(ctx.as_auth(), tx.deserialize_json::<Tx>().unwrap(), None).should_succeed();
    }
}
