use {
    alloy_dyn_abi::{Eip712Domain, TypedData},
    alloy_primitives::U160,
    anyhow::{anyhow, bail, ensure},
    base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine},
    dango_account_factory::{ACCOUNTS, ACCOUNTS_BY_USER, KEYS, OTPS},
    dango_types::{
        account_factory::Account,
        auth::{
            ClientData, Credential, Key, Metadata, OtpKey, SessionInfo, SignDoc, Signature,
            StandardCredential,
        },
        config::AppConfig,
    },
    grug::{
        json, Addr, Api, AuthCtx, AuthMode, BorshDeExt, Counter, Inner, JsonDeExt, JsonSerExt,
        Query, StdResult, Tx,
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
        let app_cfg: AppConfig = ctx.querier.query_app_config()?;
        app_cfg.addresses.account_factory
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
    // - query the key by key hash and username.
    // - query the account info.
    //
    // We use Wasm raw queries instead of smart queries to optimize on gas.
    // We also user the multi query to reduce the number of FFI calls.
    let (key, account) = {
        let [res1, res2, res3] = ctx.querier.query_multi([
            Query::wasm_raw(
                factory,
                ACCOUNTS_BY_USER.path((&metadata.username, tx.sender)),
            ),
            Query::wasm_raw(factory, KEYS.path((&metadata.username, metadata.key_hash))),
            Query::wasm_raw(factory, ACCOUNTS.path(tx.sender)),
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

        let key = res2
            .as_wasm_raw()
            .ok_or_else(|| anyhow!("key hash {} not found", metadata.key_hash))?
            .deserialize_borsh()?;

        let account: Account = res3
            .as_wasm_raw()
            .ok_or_else(|| anyhow!("account {} not found", tx.sender))?
            .deserialize_borsh()?;

        // Deserialize the key from Borsh bytes.
        (key, account)
    };

    let maybe_otp_key = if account.is_otp_enabled() {
        let otp_key = ctx
            .querier
            .query_wasm_raw(factory, OTPS.path(&metadata.username))?
            .ok_or_else(|| anyhow!("OTP key not registered for username {}.", metadata.username))?
            .deserialize_borsh::<OtpKey>()?;

        Some(otp_key)
    } else {
        None
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

    let sign_doc = SignDoc {
        sender: ctx.contract,
        messages: tx.msgs.into_inner(),
        chain_id: ctx.chain_id,
        sequence,
    };

    match ctx.mode {
        AuthMode::Check | AuthMode::Finalize => match tx.credential.deserialize_json()? {
            Credential::Standard(standard_credential) => {
                // Verify the `SignDoc` signatures.
                verify_standard_credential(
                    ctx.api,
                    standard_credential,
                    key,
                    maybe_otp_key,
                    VerifyData::SignDoc(&sign_doc),
                )?;
            },
            Credential::Session(session) => {
                ensure!(
                    session.session_info.expire_at > ctx.block.timestamp,
                    "session expired at {:?}.",
                    session.session_info.expire_at
                );

                ensure!(
                    session
                        .session_info
                        .whitelisted_accounts
                        .contains(&tx.sender),
                    "account {} not whitelisted.",
                    tx.sender
                );

                // Verify the `SessionInfo` signatures.
                verify_standard_credential(
                    ctx.api,
                    session.session_info_signature,
                    key,
                    maybe_otp_key,
                    VerifyData::SessionInfo(&session.session_info),
                )?;

                // Verify the `SignDoc` signature.
                verify_signature(
                    ctx.api,
                    Key::Secp256k1(session.session_info.session_key),
                    Signature::Secp256k1(session.session_signature),
                    &VerifyData::SignDoc(&sign_doc),
                )?;
            },
        },
        AuthMode::Simulate => (),
    };

    Ok(())
}

fn verify_standard_credential(
    api: &dyn Api,
    credential: StandardCredential,
    key: Key,
    maybe_otp_key: Option<OtpKey>,
    data: VerifyData,
) -> anyhow::Result<()> {
    verify_signature(api, key, credential.signature, &data)?;

    match (maybe_otp_key, credential.otp_signature) {
        (Some(otp_key), Some(otp_signature)) => verify_signature(
            api,
            Key::Secp256k1(otp_key.key),
            Signature::Secp256k1(otp_signature),
            &data,
        ),
        (None, None) => Ok(()),
        _ => bail!("otp key and signature must be both present or both absent"),
    }
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
                VerifyData::SignDoc(sign_doc) => (
                    Some(U160::from_be_bytes(sign_doc.sender.into_inner()).into()),
                    json!({
                        "chainId": sign_doc.chain_id,
                        "sequence": sign_doc.sequence,
                        "messages": sign_doc.messages,
                    }),
                ),

                VerifyData::SessionInfo(session_info) => (
                    None,
                    json!({
                        "session_key": session_info.session_key,
                        "expire_at": session_info.expire_at,
                        "account": session_info.whitelisted_accounts,
                    }),
                ),
            };

            let typed_data = TypedData {
                resolver,
                domain: Eip712Domain {
                    name: domain.name,
                    verifying_contract,
                    ..Default::default()
                },
                primary_type: "Message".to_string(),
                message: message.into_inner(),
            };

            // EIP-712 hash used in the signature.
            let sign_bytes = typed_data.eip712_signing_hash()?;

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
    SessionInfo(&'a SessionInfo),
    SignDoc(&'a SignDoc),
}

impl VerifyData<'_> {
    fn as_sign_bytes(&self) -> StdResult<Vec<u8>> {
        match self {
            VerifyData::SessionInfo(session_info) => session_info.to_json_vec(),
            VerifyData::SignDoc(sign_doc) => sign_doc.to_json_vec(),
        }
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_types::{
            account::single::Params,
            account_factory::{AccountParams, Username},
            config::{AppAddresses, DANGO_DENOM},
        },
        grug::{btree_map, Addr, AuthMode, Hash160, MockContext, MockQuerier},
        std::str::FromStr,
    };

    /// Address of the account factory for use in the following tests.
    const ACCOUNT_FACTORY: Addr = Addr::mock(254);

    #[test]
    fn passkey_authentication() {
        let user_address = Addr::from_str("0x4857ff85aa9d69c73bc86eb45949455b45cca580").unwrap();
        let user_username = Username::from_str("passkey").unwrap();
        let user_keyhash = Hash160::from_str("52396FDE2F222D21B62DDE0CE93BC6E109823552").unwrap();
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
                    "sig": "32jN7YGtag/NHt8FOXEEGNS7ExANwPwHi7FGel67+dBSWbLid0ZT5ew9LIuU9cFNXqJ/0xgaXuWbNhyVbCrL5g==",
                    "client_data": "eyJ0eXBlIjoid2ViYXV0aG4uZ2V0IiwiY2hhbGxlbmdlIjoicUMxYXdaNFpzS1lGMnhyYURMYlUza01VYWRGTzVMNmJvaDc0ck5zVTd5OCIsIm9yaWdpbiI6Imh0dHA6Ly9sb2NhbGhvc3Q6NTA4MCIsImNyb3NzT3JpZ2luIjpmYWxzZX0=",
                    "authenticator_data": "SZYN5YgOjGh0NBcPZHZgW4/krrmihjLHmVzzuoMdl2MZAAAAAA=="
                  }
                }
              }
            },
            "data": {
                "key_hash": "52396FDE2F222D21B62DDE0CE93BC6E109823552",
                "username": "passkey",
                "sequence": 0
            },
            "msgs": [{
                "transfer": {
                  "to": "0x064c5e20b422b5d817fe800119dac0ab43b17a80",
                  "coins": {
                        "uusdc": "1000000"
                  }
                }
            }],
            "gas_limit": 2602525
        }"#;

        let querier = MockQuerier::new()
            .with_app_config(AppConfig {
                dango: DANGO_DENOM.clone(),
                addresses: AppAddresses {
                    account_factory: ACCOUNT_FACTORY,
                    // Address below don't matter for this test.
                    ibc_transfer: Addr::mock(0),
                    oracle: Addr::mock(1),
                    lending: Addr::mock(0), // doesn't matter for this test
                },
                collateral_powers: btree_map! {},
            })
            .unwrap()
            .with_raw_contract_storage(ACCOUNT_FACTORY, |storage| {
                ACCOUNTS_BY_USER
                    .insert(storage, (&user_username, user_address))
                    .unwrap();
                KEYS.save(storage, (&user_username, user_keyhash), &user_key)
                    .unwrap();
                ACCOUNTS
                    .save(storage, user_address, &Account {
                        index: 5,
                        params: AccountParams::Spot(Params::new(user_username)),
                    })
                    .unwrap()
            });

        let mut ctx = MockContext::new()
            .with_querier(querier)
            .with_contract(user_address)
            .with_chain_id("dev-3")
            .with_mode(AuthMode::Finalize);

        authenticate_tx(ctx.as_auth(), tx.deserialize_json().unwrap(), None, None).unwrap();
    }

    #[test]
    fn eip712_authentication() {
        let user_address = Addr::from_str("0x227e7e3d56ffd984ba6e3ead892f5676fa722a16").unwrap();
        let user_username = Username::from_str("javier_1").unwrap();
        let user_keyhash = Hash160::from_str("904EF73D090935DB7DB7AE7162DB546268225D66").unwrap();
        let user_key = Key::Secp256k1(
            [
                3, 115, 37, 57, 128, 37, 222, 189, 9, 42, 142, 196, 85, 27, 226, 112, 136, 195,
                174, 6, 40, 39, 221, 182, 179, 146, 169, 207, 108, 218, 67, 27, 71,
            ]
            .into(),
        );

        let querier = MockQuerier::new()
            .with_app_config(AppConfig {
                dango: DANGO_DENOM.clone(),
                addresses: AppAddresses {
                    account_factory: ACCOUNT_FACTORY,
                    // Address below don't matter for this test.
                    ibc_transfer: Addr::mock(0),
                    oracle: Addr::mock(1),
                    lending: Addr::mock(0), // doesn't matter for this test
                },
                collateral_powers: btree_map! {},
            })
            .unwrap()
            .with_raw_contract_storage(ACCOUNT_FACTORY, |storage| {
                ACCOUNTS_BY_USER
                    .insert(storage, (&user_username, user_address))
                    .unwrap();
                KEYS.save(storage, (&user_username, user_keyhash), &user_key)
                    .unwrap();
                ACCOUNTS
                    .save(storage, user_address, &Account {
                        index: 5,
                        params: AccountParams::Spot(Params::new(user_username)),
                    })
                    .unwrap()
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
                    "sig": "3QfufLebIDnA/FlT3yV65yCivI4s5tCF1Rluq5Q+cYQwTqH9HDyXU/XcwtX5/4H0GFZ26CkfvPFYlx6m3lRKcw==",
                    "typed_data": "eyJ0eXBlcyI6eyJFSVA3MTJEb21haW4iOlt7Im5hbWUiOiJuYW1lIiwidHlwZSI6InN0cmluZyJ9LHsibmFtZSI6InZlcmlmeWluZ0NvbnRyYWN0IiwidHlwZSI6ImFkZHJlc3MifV0sIk1lc3NhZ2UiOlt7Im5hbWUiOiJjaGFpbklkIiwidHlwZSI6InN0cmluZyJ9LHsibmFtZSI6InNlcXVlbmNlIiwidHlwZSI6InVpbnQzMiJ9LHsibmFtZSI6Im1lc3NhZ2VzIiwidHlwZSI6IlR4TWVzc2FnZVtdIn1dLCJUeE1lc3NhZ2UiOlt7Im5hbWUiOiJ0cmFuc2ZlciIsInR5cGUiOiJUcmFuc2ZlciJ9XSwiVHJhbnNmZXIiOlt7Im5hbWUiOiJ0byIsInR5cGUiOiJhZGRyZXNzIn0seyJuYW1lIjoiY29pbnMiLCJ0eXBlIjoiQ29pbnMifV0sIkNvaW5zIjpbeyJuYW1lIjoidXVzZGMiLCJ0eXBlIjoic3RyaW5nIn1dfSwicHJpbWFyeVR5cGUiOiJNZXNzYWdlIiwiZG9tYWluIjp7Im5hbWUiOiJsb2NhbGhvc3QiLCJ2ZXJpZnlpbmdDb250cmFjdCI6IjB4MjI3ZTdlM2Q1NmZmZDk4NGJhNmUzZWFkODkyZjU2NzZmYTcyMmExNiJ9LCJtZXNzYWdlIjp7ImNoYWluSWQiOiJkZXYtMyIsInNlcXVlbmNlIjowLCJtZXNzYWdlcyI6W3sidHJhbnNmZXIiOnsidG8iOiIweDA2NGM1ZTIwYjQyMmI1ZDgxN2ZlODAwMTE5ZGFjMGFiNDNiMTdhODAiLCJjb2lucyI6eyJ1dXNkYyI6IjEwMDAwMDAifX19XX19"
                  }
                }
              }
            },
            "data": {
                "key_hash": "904EF73D090935DB7DB7AE7162DB546268225D66",
                "username": "javier_1",
                "sequence": 0
            },
            "msgs": [{
              "transfer": {
                "to": "0x064c5e20b422b5d817fe800119dac0ab43b17a80",
                "coins": {
                  "uusdc": "1000000"
                }
              }
            }],
            "gas_limit": 2647711
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
        let user_address = Addr::from_str("0xb86b2d96971c32f68241df04691479edb6a9cd3b").unwrap();
        let user_username = Username::from_str("owner").unwrap();
        let user_keyhash = Hash160::from_str("57D9205BFB0ED62C0667462E07EAF1AA31228DD4").unwrap();
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
                "secp256k1": "6fViCRV4+NEs7AiiF2o7yWBar3IKu4S1tnxDCtB71J8XnaR69IRyvURLIH4HAc0APK3DA8Vy8vEtpaEzlllqKg=="
                }
            }
        },
        "data": {
            "key_hash": "57D9205BFB0ED62C0667462E07EAF1AA31228DD4",
            "username": "owner",
            "sequence": 0
        },
        "msgs": [{
            "transfer": {
                "to": "0x064c5e20b422b5d817fe800119dac0ab43b17a80",
                "coins": {
                    "uusdc": "10000"
                }
            }
        }],
        "gas_limit": 2647275
        }"#;

        let querier = MockQuerier::new()
            .with_app_config(AppConfig {
                dango: DANGO_DENOM.clone(),
                addresses: AppAddresses {
                    account_factory: ACCOUNT_FACTORY,
                    // Address below don't matter for this test.
                    ibc_transfer: Addr::mock(0),
                    oracle: Addr::mock(1),
                    lending: Addr::mock(0), // doesn't matter for this test
                },
                collateral_powers: btree_map! {},
            })
            .unwrap()
            .with_raw_contract_storage(ACCOUNT_FACTORY, |storage| {
                ACCOUNTS_BY_USER
                    .insert(storage, (&user_username, user_address))
                    .unwrap();

                KEYS.save(storage, (&user_username, user_keyhash), &user_key)
                    .unwrap();
                ACCOUNTS
                    .save(storage, user_address, &Account {
                        index: 5,
                        params: AccountParams::Spot(Params::new(
                            Username::from_str("owner").unwrap(),
                        )),
                    })
                    .unwrap()
            });

        let mut ctx = MockContext::new()
            .with_querier(querier)
            .with_contract(user_address)
            .with_chain_id("dev-3")
            .with_mode(AuthMode::Finalize);

        authenticate_tx(ctx.as_auth(), tx.deserialize_json().unwrap(), None, None).unwrap();
    }
}
