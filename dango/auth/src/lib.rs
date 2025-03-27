use {
    alloy::{
        dyn_abi::{Eip712Domain, TypedData},
        primitives::U160,
    },
    anyhow::{bail, ensure},
    base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD},
    dango_types::{
        DangoQuerier,
        auth::{
            ClientData, Credential, Key, Metadata, Nonce, SignDoc, Signature, StandardCredential,
        },
    },
    grug::{
        Addr, Api, AuthCtx, AuthMode, Inner, Item, Json, JsonDeExt, JsonSerExt, QuerierExt,
        SignData, StdError, StdResult, Storage, StorageQuerier, Tx, json,
    },
    sha2::Sha256,
    std::collections::BTreeSet,
};

/// The expected storage layout of the account factory contract.
pub mod account_factory {
    use {
        dango_types::{account_factory::Username, auth::Key},
        grug::{Addr, Hash256, Map, Set},
    };

    pub const KEYS: Map<(&Username, Hash256), Key> = Map::new("key");

    pub const ACCOUNTS_BY_USER: Set<(&Username, Addr)> = Set::new("account__user");
}

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
                account_factory::ACCOUNTS_BY_USER.path((&metadata.username, tx.sender)),
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
            let key = ctx.querier.query_wasm_path(
                factory,
                &account_factory::KEYS.path((&metadata.username, key_hash)),
            )?;

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
                    &VerifyData::Arbitrary(&json!({
                        "session_key": session.session_info.session_key,
                        "expire_at": session.session_info.expire_at,
                    })),
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

pub fn verify_signature(
    api: &dyn Api,
    key: Key,
    signature: Signature,
    data: &VerifyData,
) -> anyhow::Result<()> {
    match (key, signature) {
        (Key::Ethereum(addr), Signature::Eip712(cred)) => {
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
                VerifyData::Arbitrary(json) => (None, json.to_owned().clone()),
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

            // The first 64 bytes of the Ethereum signature is the typical
            // Secp256k1 signature, while the last byte is the recovery ID.
            //
            // In Ethereum, the recovery ID is usually 27 or 28, instead of the
            // standard 0 or 1 used in raw ECDSA recoverable signatures.
            // However, our `Api` implementation should handle this, so no action
            // is needed here.
            let signature = &cred.sig[0..64];
            let recovery_id = cred.sig[64];

            // Recover the Ethereum public key from the signature.
            let pk = api.secp256k1_pubkey_recover(&sign_bytes.0, signature, recovery_id, false)?;

            // Derive Ethereum address from the public key.
            let recovered_addr = &api.keccak256(&pk[1..])[12..];

            ensure!(
                addr.as_ref() == recovered_addr,
                "recovered Ethereum address does not match: {} != {}",
                addr,
                hex::encode(recovered_addr)
            );
        },
        (Key::Secp256r1(pk), Signature::Passkey(cred)) => {
            let signed_hash = {
                let client_data: ClientData = cred.client_data.deserialize_json()?;

                let sign_data = data.to_sign_data()?;
                let sign_data_base64 = URL_SAFE_NO_PAD.encode(sign_data);

                ensure!(
                    client_data.challenge == sign_data_base64,
                    "incorrect challenge: expecting {}, got {}",
                    sign_data_base64,
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
            let sign_data = data.to_sign_data()?;

            api.secp256k1_verify(&sign_data, &sig, &pk)?;
        },
        _ => bail!("key and credential types don't match!"),
    }
    Ok(())
}

pub enum VerifyData<'a> {
    Standard {
        sign_doc: SignDoc,
        chain_id: String,
        nonce: Nonce,
    },
    Arbitrary(&'a Json),
}

impl SignData for VerifyData<'_> {
    type Error = StdError;
    type Hasher = Sha256;

    fn to_prehash_sign_data(&self) -> StdResult<Vec<u8>> {
        match self {
            VerifyData::Standard { sign_doc, .. } => sign_doc.to_prehash_sign_data(),
            VerifyData::Arbitrary(json) => json.to_json_vec(),
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
        grug::{Addr, AuthMode, Hash256, MockContext, MockQuerier, ResultExt, btree_map},
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
          "credential": {
            "standard": {
              "key_hash": "8E60264C2887C814C0C1E873A66F51F294149EFC3161CB1A195277D330927F31",
              "signature": {
                "passkey": {
                  "authenticator_data": "SZYN5YgOjGh0NBcPZHZgW4/krrmihjLHmVzzuoMdl2MZAAAAAA==",
                  "client_data": "eyJ0eXBlIjoid2ViYXV0aG4uZ2V0IiwiY2hhbGxlbmdlIjoiS1I3OXRVWHp4R2liTzloVmhQNlk0TmZGcmRsOHg0dVR4cm9RbU5HTGhzayIsIm9yaWdpbiI6Imh0dHA6Ly9sb2NhbGhvc3Q6NTA4MCIsImNyb3NzT3JpZ2luIjpmYWxzZSwib3RoZXJfa2V5c19jYW5fYmVfYWRkZWRfaGVyZSI6ImRvIG5vdCBjb21wYXJlIGNsaWVudERhdGFKU09OIGFnYWluc3QgYSB0ZW1wbGF0ZS4gU2VlIGh0dHBzOi8vZ29vLmdsL3lhYlBleCJ9",
                  "sig": "fTKkzapyn0e3Q27ARsdxTGDQA0rSv/hmvSp++xJdKk4yBgP4CxqidByWOA0FmVQ2wBuob9BINpu7Eho+UFFroQ=="
                }
              }
            }
          },
          "data": {
            "chain_id": "dev-5",
            "nonce": 0,
            "username": "pass_local"
          },
          "gas_limit": 2448139,
          "msgs": [
            {
              "transfer": {
                "0x33361de42571d6aa20c37daa6da4b5ab67bfaad9": {
                  "hyp/eth/usdc": "1000000"
                }
              }
            }
          ],
          "sender": "0x94e4e04fbf35a0e67c559fe1c9579de9fdd0f6ed"
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
                account_factory::ACCOUNTS_BY_USER
                    .insert(storage, (&user_username, user_address))
                    .unwrap();
                account_factory::KEYS
                    .save(storage, (&user_username, user_keyhash), &user_key)
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
            Hash256::from_str("7D8FB7895BEAE0DF16E3E5F6FA7EB10CDE735E5B7C9A79DFCD8DD32A6BDD2165")
                .unwrap();
        let user_key =
            Key::Ethereum(Addr::from_str("0x4c9d879264227583f49af3c99eb396fe4735a935").unwrap());

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
                account_factory::ACCOUNTS_BY_USER
                    .insert(storage, (&user_username, user_address))
                    .unwrap();
                account_factory::KEYS
                    .save(storage, (&user_username, user_keyhash), &user_key)
                    .unwrap();
            });

        let mut ctx = MockContext::new()
            .with_querier(querier)
            .with_contract(user_address)
            .with_chain_id("dev-6")
            .with_mode(AuthMode::Finalize);

        let tx = r#"{
          "credential": {
            "standard": {
              "key_hash": "7D8FB7895BEAE0DF16E3E5F6FA7EB10CDE735E5B7C9A79DFCD8DD32A6BDD2165",
              "signature": {
                "eip712": {
                  "sig": "qhvraAO/AnyJO621ig38y8Rc4k12UtuKurqTjMOCHogR5UpyptvZ2lTl0gH0wjFiUTNp3uFe+WAh760HWq2Mnxs=",
                  "typed_data": "eyJ0eXBlcyI6eyJFSVA3MTJEb21haW4iOlt7Im5hbWUiOiJuYW1lIiwidHlwZSI6InN0cmluZyJ9LHsibmFtZSI6InZlcmlmeWluZ0NvbnRyYWN0IiwidHlwZSI6ImFkZHJlc3MifV0sIk1lc3NhZ2UiOlt7Im5hbWUiOiJtZXRhZGF0YSIsInR5cGUiOiJNZXRhZGF0YSJ9LHsibmFtZSI6Imdhc19saW1pdCIsInR5cGUiOiJ1aW50MzIifSx7Im5hbWUiOiJtZXNzYWdlcyIsInR5cGUiOiJUeE1lc3NhZ2VbXSJ9XSwiTWV0YWRhdGEiOlt7Im5hbWUiOiJ1c2VybmFtZSIsInR5cGUiOiJzdHJpbmcifSx7Im5hbWUiOiJjaGFpbl9pZCIsInR5cGUiOiJzdHJpbmcifSx7Im5hbWUiOiJub25jZSIsInR5cGUiOiJ1aW50MzIifV0sIlR4TWVzc2FnZSI6W3sibmFtZSI6InRyYW5zZmVyIiwidHlwZSI6IlRyYW5zZmVyIn1dLCJUcmFuc2ZlciI6W3sibmFtZSI6IjB4MzMzNjFkZTQyNTcxZDZhYTIwYzM3ZGFhNmRhNGI1YWI2N2JmYWFkOSIsInR5cGUiOiJDb2luMCJ9XSwiQ29pbjAiOlt7Im5hbWUiOiJoeXAvZXRoL3VzZGMiLCJ0eXBlIjoic3RyaW5nIn1dfSwicHJpbWFyeVR5cGUiOiJNZXNzYWdlIiwiZG9tYWluIjp7Im5hbWUiOiJkYW5nbyIsInZlcmlmeWluZ0NvbnRyYWN0IjoiMHhiNjYyMjdjZjRlYTgwMGI2YjE5YWVkMTk4Mzk1ZmQwYTJkODBlZTFkIn0sIm1lc3NhZ2UiOnsibWV0YWRhdGEiOnsiY2hhaW5faWQiOiJkZXYtNiIsInVzZXJuYW1lIjoiamF2aWVyIiwibm9uY2UiOjB9LCJnYXNfbGltaXQiOjI0NDgxMzksIm1lc3NhZ2VzIjpbeyJ0cmFuc2ZlciI6eyIweDMzMzYxZGU0MjU3MWQ2YWEyMGMzN2RhYTZkYTRiNWFiNjdiZmFhZDkiOnsiaHlwL2V0aC91c2RjIjoiMTAwMDAwMCJ9fX1dfX0="
                }
              }
            }
          },
          "data": {
            "chain_id": "dev-6",
            "nonce": 0,
            "username": "javier"
          },
          "gas_limit": 2448139,
          "msgs": [
            {
              "transfer": {
                "0x33361de42571d6aa20c37daa6da4b5ab67bfaad9": {
                  "hyp/eth/usdc": "1000000"
                }
              }
            }
          ],
          "sender": "0xb66227cf4ea800b6b19aed198395fd0a2d80ee1d"
        }"#;

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
          "credential": {
            "standard": {
              "key_hash": "06E54A648823A1F12E1F03FED193C9FE0C030A65507FF09066BF9E067CD375D2",
              "signature": {
                "secp256k1": "CLlermDLySBkXKiU33LTPtzeOt8Rp0W7bKs3nMdRbEZUDumK7fldZ6WTxCjvg7apTPO1dxqFUzsbvwfQFaxG+w=="
              }
            }
          },
          "data": {
            "chain_id": "dev-6",
            "nonce": 0,
            "username": "owner"
          },
          "gas_limit": 2448142,
          "msgs": [
            {
              "transfer": {
                "0xb66227cf4ea800b6b19aed198395fd0a2d80ee1d": {
                  "hyp/eth/usdc": "100"
                }
              }
            }
          ],
          "sender": "0x33361de42571d6aa20c37daa6da4b5ab67bfaad9"
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
                account_factory::ACCOUNTS_BY_USER
                    .insert(storage, (&user_username, user_address))
                    .unwrap();
                account_factory::KEYS
                    .save(storage, (&user_username, user_keyhash), &user_key)
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
    fn session_key_with_passkey_authentication() {
        let user_address = Addr::from_str("0x5614a130eb9322e549e0d86d24a7bb1a7f683b28").unwrap();
        let user_username = Username::from_str("pass_local").unwrap();
        let user_keyhash =
            Hash256::from_str("010AB8AAF008DA93DB00F94D818931832F54192A334D933629768B59A2932817")
                .unwrap();
        let user_key = Key::Secp256r1(
            [
                3, 49, 131, 213, 54, 16, 255, 178, 137, 198, 32, 99, 238, 21, 5, 25, 52, 140, 150,
                228, 146, 68, 250, 57, 250, 251, 135, 159, 84, 162, 229, 40, 155,
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
                account_factory::ACCOUNTS_BY_USER
                    .insert(storage, (&user_username, user_address))
                    .unwrap();
                account_factory::KEYS
                    .save(storage, (&user_username, user_keyhash), &user_key)
                    .unwrap();
            });

        let mut ctx = MockContext::new()
            .with_querier(querier)
            .with_contract(user_address)
            .with_chain_id("dev-6")
            .with_mode(AuthMode::Finalize);

        let tx = r#"{
          "credential": {
            "session": {
              "authorization": {
                "key_hash": "010AB8AAF008DA93DB00F94D818931832F54192A334D933629768B59A2932817",
                "signature": {
                  "passkey": {
                    "authenticator_data": "SZYN5YgOjGh0NBcPZHZgW4/krrmihjLHmVzzuoMdl2MZAAAAAA==",
                    "client_data": "eyJ0eXBlIjoid2ViYXV0aG4uZ2V0IiwiY2hhbGxlbmdlIjoiTlMwTFVJUUZpUC1SN01MSmE5V3RBbEttcUZhcWdfbTdqTzZaeExubE1SZyIsIm9yaWdpbiI6Imh0dHA6Ly9sb2NhbGhvc3Q6NTA4MCIsImNyb3NzT3JpZ2luIjpmYWxzZSwib3RoZXJfa2V5c19jYW5fYmVfYWRkZWRfaGVyZSI6ImRvIG5vdCBjb21wYXJlIGNsaWVudERhdGFKU09OIGFnYWluc3QgYSB0ZW1wbGF0ZS4gU2VlIGh0dHBzOi8vZ29vLmdsL3lhYlBleCJ9",
                    "sig": "kutvF0E0eD+K0FCD575y1HuaToPrdBFB20VIlxiA4HeKHdXwvDjKfcMPSnV752jb9xEeBvO1Jym+Z7PJR3dfeg=="
                  }
                }
              },
              "session_info": {
                "expire_at": "1743109311084",
                "session_key": "A9q7FcgFOItKcmXpqTZZyAgTLqszNCdG/LkHF+UZyBMs"
              },
              "session_signature": "lBPASUUQyv+YaQg0b/1XGvqn7Iuk7R5uGsh9m1m/IdU62YQHl+VT2ZURQ4GsIIWer9oHevALgMGqjA1KraW21A=="
            }
          },
          "data": {
            "chain_id": "dev-6",
            "nonce": 0,
            "username": "pass_local"
          },
          "gas_limit": 2448139,
          "msgs": [
            {
              "transfer": {
                "0x33361de42571d6aa20c37daa6da4b5ab67bfaad9": {
                  "hyp/eth/usdc": "1000000"
                }
              }
            }
          ],
          "sender": "0x5614a130eb9322e549e0d86d24a7bb1a7f683b28"
        }"#;

        authenticate_tx(ctx.as_auth(), tx.deserialize_json::<Tx>().unwrap(), None).should_succeed();
    }

    #[test]
    fn session_key_with_eip712_authentication() {
        let user_address = Addr::from_str("0xb66227cf4ea800b6b19aed198395fd0a2d80ee1d").unwrap();
        let user_username = Username::from_str("javier").unwrap();
        let user_keyhash =
            Hash256::from_str("7D8FB7895BEAE0DF16E3E5F6FA7EB10CDE735E5B7C9A79DFCD8DD32A6BDD2165")
                .unwrap();
        let user_key =
            Key::Ethereum(Addr::from_str("0x4c9d879264227583f49af3c99eb396fe4735a935").unwrap());

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
                account_factory::ACCOUNTS_BY_USER
                    .insert(storage, (&user_username, user_address))
                    .unwrap();
                account_factory::KEYS
                    .save(storage, (&user_username, user_keyhash), &user_key)
                    .unwrap();
            });

        let mut ctx = MockContext::new()
            .with_querier(querier)
            .with_contract(user_address)
            .with_chain_id("dev-6")
            .with_mode(AuthMode::Finalize);

        let tx = r#"{
          "credential": {
            "session": {
              "authorization": {
                "key_hash": "7D8FB7895BEAE0DF16E3E5F6FA7EB10CDE735E5B7C9A79DFCD8DD32A6BDD2165",
                "signature": {
                  "eip712": {
                    "sig": "2JSrtr1cB6bEVxio6xNCb4z3G7JZo3cF2FF3h6GRSTZVI3Qrqme4wyNUseKrG8J/Mo/DxwzYcj6IlcQnWENtNRs=",
                    "typed_data": "eyJkb21haW4iOnsibmFtZSI6IkRhbmdvQXJiaXRyYXJ5TWVzc2FnZSJ9LCJtZXNzYWdlIjp7InNlc3Npb25fa2V5IjoiQThiWDVhbU4yNGlXMDV4b2c2SGVLWXJ3THA5NU9qWStZcW1iUlQ3U0twZVgiLCJleHBpcmVfYXQiOiIxNzQzMTA5NjkzNjM3In0sInByaW1hcnlUeXBlIjoiTWVzc2FnZSIsInR5cGVzIjp7IkVJUDcxMkRvbWFpbiI6W3sibmFtZSI6Im5hbWUiLCJ0eXBlIjoic3RyaW5nIn1dLCJNZXNzYWdlIjpbeyJuYW1lIjoic2Vzc2lvbl9rZXkiLCJ0eXBlIjoic3RyaW5nIn0seyJuYW1lIjoiZXhwaXJlX2F0IiwidHlwZSI6InN0cmluZyJ9XX19"
                  }
                }
              },
              "session_info": {
                "expire_at": "1743109693637",
                "session_key": "A8bX5amN24iW05xog6HeKYrwLp95OjY+YqmbRT7SKpeX"
              },
              "session_signature": "510PITf+ZKexzi+g+5J5SS1JkvKBeMoUvBNh7h9viTcGCMvdkgNvdJsdOGpJcG19XYsgPIaMSKZyHEQkVUK2DQ=="
            }
          },
          "data": {
            "chain_id": "dev-6",
            "nonce": 0,
            "username": "javier"
          },
          "gas_limit": 2448139,
          "msgs": [
            {
              "transfer": {
                "0x33361de42571d6aa20c37daa6da4b5ab67bfaad9": {
                  "hyp/eth/usdc": "1000000"
                }
              }
            }
          ],
          "sender": "0xb66227cf4ea800b6b19aed198395fd0a2d80ee1d"
        }"#;

        authenticate_tx(ctx.as_auth(), tx.deserialize_json::<Tx>().unwrap(), None).should_succeed();
    }
}
