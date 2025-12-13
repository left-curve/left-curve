use {
    alloy::{
        dyn_abi::{Eip712Domain, TypedData},
        primitives::{U160, U256, address, uint},
    },
    anyhow::{bail, ensure},
    dango_types::{
        DangoQuerier,
        account_factory::RegisterUserData,
        auth::{
            AccountStatus, ClientData, Credential, Key, Metadata, Nonce, SessionInfo, SignDoc,
            Signature, StandardCredential,
        },
    },
    data_encoding::BASE64URL_NOPAD,
    grug::{
        Addr, Api, AuthCtx, AuthMode, Coins, GENESIS_BLOCK_HEIGHT, Inner, JsonDeExt, JsonSerExt,
        MutableCtx, QuerierExt, SignData, StdError, StdResult, Storage, StorageQuerier, Tx,
    },
    sha2::Sha256,
    std::collections::BTreeSet,
};

/// The expected storage layout of the account factory contract.
pub mod account_factory {
    use {
        dango_types::{account_factory::UserIndex, auth::Key},
        grug::{Addr, Hash256, Map, Set},
    };

    pub const KEYS: Map<(UserIndex, Hash256), Key> = Map::new("key");

    pub const ACCOUNTS_BY_USER: Set<(UserIndex, Addr)> = Set::new("account__user");
}

/// The expected storage layout of the account contract.
pub mod account {
    use {
        dango_types::auth::{AccountStatus, Nonce},
        grug::Item,
        std::collections::BTreeSet,
    };

    /// The account's status. Only accounts in the `Active` state can send
    /// transactions.
    ///
    /// Upon creation, an account is initialized to the `Inactive` state. It
    /// must receive transfer equal to or greater than the minimum deposit
    /// (specified in the app-config) to become `Active`.
    ///
    /// If this storage slot is empty, it's default to the `Inactive` state.
    pub const STATUS: Item<AccountStatus> = Item::new("status");

    /// The most recent nonces that have been used to send transactions.
    ///
    /// Both account types (single, multi) stores their nonces in this same
    /// storage slot.
    pub const SEEN_NONCES: Item<BTreeSet<Nonce>> = Item::new("seen_nonces");
}

/// The [EIP-155](https://github.com/ethereum/EIPs/blob/master/EIPS/eip-155.md)
/// chain ID of Ethereum mainnet.
///
/// The [EIP-712](https://github.com/ethereum/EIPs/blob/master/EIPS/eip-712.md#definition-of-domainseparator)
/// standard requires the `chainId` field in the domain. Some wallets enforce
/// this requirement.
///
/// Since Dango isn't an EVM chain and hence doesn't have an EIP-155 chain ID,
/// we use that of Ethereum mainnet for compatibility.
pub const EIP155_CHAIN_ID: U256 = uint!(0x1_U256);

/// Max number of tracked nonces.
pub const MAX_SEEN_NONCES: usize = 20;

/// The maximum difference betwen the nonce of an incoming transaction, and the
/// biggest seen nonce so far.
///
/// This is to prevent a specific DoS attack. A rogue member of a multisig can
/// submit a batch of transactions, such that the `SEEN_NONCES` set is fully
/// filled with the following nonces:
///
/// ```plain
/// (u32::MAX - MAX_SEEN_NONCES + 1)..=u32::MAX
/// ```
///
/// This prevents the multisig from being able to submit any more transactions,
/// because a new tx must come with a nonce bigger than `u32::MAX`, which is
/// impossible.
///
/// We prevent this attack by requiring the nonce of a new tx must not be too
/// much bigger than the biggest nonce seen so far.
pub const MAX_NONCE_INCREASE: Nonce = 100;

/// Query the account's status.
pub fn query_status(storage: &dyn Storage) -> StdResult<AccountStatus> {
    account::STATUS
        .may_load(storage)
        .map(|opt| opt.unwrap_or_default()) // default to to `Inactive` state
}

/// Query the set of most recent nonce tracked.
pub fn query_seen_nonces(storage: &dyn Storage) -> StdResult<BTreeSet<Nonce>> {
    account::SEEN_NONCES
        .may_load(storage)
        .map(|opt| opt.unwrap_or_default()) // default to an empty B-tree set
}

pub fn create_account(ctx: MutableCtx) -> anyhow::Result<()> {
    let app_cfg = ctx.querier.query_dango_config()?;

    // Only the account factory can create new accounts.
    ensure!(
        ctx.sender == app_cfg.addresses.account_factory,
        "you don't have the right, O you don't have the right"
    );

    // Upon creation, the account's status is set to `Inactive`.
    // We don't need to save it in storage, because if storage is empty, it's
    // default to `Inactive`. This is an intentional optimization to minimize
    // disk writes.
    //
    // Two exceptions to this are:
    // 1. during genesis;
    // 2. the account received sufficient funds during instantiation.
    // In these cases, activate the account now.
    if ctx.block.height == GENESIS_BLOCK_HEIGHT
        || is_sufficient(&ctx.funds, &app_cfg.minimum_deposit)
    {
        account::STATUS.save(ctx.storage, &AccountStatus::Active)?;
        // TODO: emit an event?
    }

    Ok(())
}

pub fn receive_transfer(ctx: MutableCtx) -> anyhow::Result<()> {
    match query_status(ctx.storage)? {
        // If the account is inactive: query the minimum deposit from app-config.
        // Activate the account is the deposit is sufficient.
        AccountStatus::Inactive => {
            let minimum = ctx.querier.query_minimum_deposit()?;
            if is_sufficient(&ctx.funds, &minimum) {
                account::STATUS.save(ctx.storage, &AccountStatus::Active)?;
                // TODO: emit an event?
            }
        },
        AccountStatus::Frozen => bail!(
            "account {} is frozen, can't receive transfers",
            ctx.contract
        ),
        AccountStatus::Active => { /* nothing to do */ },
    }

    Ok(())
}

/// A deposit is considered **sufficient** if _either_ of the following is true:
/// - the minimum deposit is zero;
/// - _any_ of the coins received has an amount greater than the minimum.
fn is_sufficient(deposit: &Coins, minimum: &Coins) -> bool {
    minimum.is_empty()
        || minimum
            .iter()
            .any(|coin| deposit.amount_of(coin.denom) >= *coin.amount)
}

/// Authenticate a transaction by ensuring:
///
/// - the username is associated with the sender account;
/// - the nonce is acceptible;
/// - the signature is authentic.
///
/// This logic is used by single-signature accounts.
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

    // If the sender account is associated with the user index, then an entry
    // must exist in the `ACCOUNTS_BY_USER` set, and the value should be empty
    // because we Borsh for encoding.
    ensure!(
        ctx.querier
            .query_wasm_raw(
                factory,
                account_factory::ACCOUNTS_BY_USER.path((metadata.user_index, tx.sender)),
            )?
            .is_some_and(|bytes| bytes.is_empty()),
        "account {} isn't associated with user {}",
        tx.sender,
        metadata.user_index,
    );

    // The account must be in the `Active` state.
    ensure!(
        query_status(ctx.storage)? == AccountStatus::Active,
        "account {} is not active",
        tx.sender
    );

    verify_nonce_and_signature(ctx, tx, Some(factory), Some(metadata))
}

/// Ensure the nonce is acceptable and the signature is authentic.
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
            account::SEEN_NONCES.may_update(ctx.storage, |maybe_nonces| {
                let mut nonces = maybe_nonces.unwrap_or_default();

                if let Some(&first) = nonces.first() {
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
                } else {
                    // Ensure the first nonce is close to zero.
                    ensure!(
                        metadata.nonce < MAX_NONCE_INCREASE,
                        "first nonce is too big: {} >= MAX_NONCE_INCREASE ({})",
                        metadata.nonce,
                        MAX_NONCE_INCREASE
                    );
                }

                // The nonce must not be too much bigger than the biggest nonce
                // seen so far.
                //
                // See the documentation for `MAX_NONCE_INCREASE` for the rationale.
                if let Some(max) = nonces.last() {
                    ensure!(
                        metadata.nonce <= max + MAX_NONCE_INCREASE,
                        "nonce is too far ahead: {} > {} + MAX_NONCE_INCREASE ({})",
                        metadata.nonce,
                        max,
                        MAX_NONCE_INCREASE
                    );
                }

                nonces.insert(metadata.nonce);

                Ok(nonces)
            })?;

            // Verify tx expiration.
            if let Some(expiry) = metadata.expiry {
                ensure!(
                    expiry > ctx.block.timestamp,
                    "transaction expired at {expiry:?}"
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

            // Query the key by key hash and user index.
            let key = ctx.querier.query_wasm_path(
                factory,
                &account_factory::KEYS.path((metadata.user_index, key_hash)),
            )?;

            if let Some(session) = session_credential {
                ensure!(
                    session.session_info.expire_at > ctx.block.timestamp,
                    "session expired at {:?}.",
                    session.session_info.expire_at
                );

                // Verify the `SessionInfo` signature.
                //
                // TODO: we can consider saving authorized session keys in the
                // contract, so it's not necessary to verify them again.
                verify_signature(
                    ctx.api,
                    key,
                    signature,
                    VerifyData::Session(session.session_info.clone()),
                )?;

                // Verify the `SignDoc` signature.
                verify_signature(
                    ctx.api,
                    Key::Secp256k1(session.session_info.session_key),
                    Signature::Secp256k1(session.session_signature),
                    VerifyData::Transaction(sign_doc),
                )?;
            } else {
                // Verify the `SignDoc` signature.
                verify_signature(ctx.api, key, signature, VerifyData::Transaction(sign_doc))?;
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
    data: VerifyData,
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
                VerifyData::Transaction(sign_doc) => (
                    Some(U160::from_be_bytes(sign_doc.sender.into_inner()).into()),
                    sign_doc.to_json_value()?,
                ),
                // The EIP-712 standard requires the `verifyingContract` field
                // in the domain. Some wallets enforce this requirement.
                // We use the zero address (0x00...00) as a placeholder for
                // these cases, indicating the signature's 'arbitrary' nature.
                VerifyData::Session(session_info) => (
                    Some(address!("0x0000000000000000000000000000000000000000")),
                    session_info.to_json_value()?,
                ),
                VerifyData::Onboard(data) => (
                    Some(address!("0x0000000000000000000000000000000000000000")),
                    data.to_json_value()?,
                ),
            };

            // EIP-712 hash used in the signature.
            let sign_bytes = TypedData {
                resolver,
                domain: Eip712Domain {
                    name: domain.name,
                    // We use Ethereum's EIP-155 chainId (0x1) for compatibility.
                    chain_id: Some(EIP155_CHAIN_ID),
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
                let sign_data_base64 = BASE64URL_NOPAD.encode(&sign_data);

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

/// The type of data that was signed.
pub enum VerifyData {
    /// The signature is for sending a transaction.
    ///
    /// To do this, the user must sign a `SignDoc` using either their primary
    /// key or a session key that has been authorized.
    Transaction(SignDoc),
    /// The signature is for authorizing a session key to send transactions
    /// on behalf on the primary key.
    ///
    /// To do this, the user must sign a `SessionInfo`.
    Session(SessionInfo),
    /// The signature is for onboarding a new user.
    ///
    /// To do this, the user must sign a `RegisterUserData`.
    Onboard(RegisterUserData),
}

impl SignData for VerifyData {
    type Error = StdError;
    type Hasher = Sha256;

    fn to_prehash_sign_data(&self) -> StdResult<Vec<u8>> {
        match self {
            VerifyData::Transaction(sign_doc) => sign_doc.to_prehash_sign_data(),
            VerifyData::Session(session_info) => session_info.to_prehash_sign_data(),
            VerifyData::Onboard(data) => data.to_prehash_sign_data(),
        }
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_types::config::{AppAddresses, AppConfig},
        grug::{
            Addr, AuthMode, Hash256, MockContext, MockQuerier, MockStorage, ResultExt, addr, hash,
        },
        hex_literal::hex,
        std::str::FromStr,
    };

    /// Address of the account factory for use in the following tests.
    const ACCOUNT_FACTORY: Addr = Addr::mock(254);

    #[test]
    fn passkey_authentication() {
        let user_address = Addr::from_str("0xd7b73f486c66fa6daecd67d7aee46a26513b07c2").unwrap();
        let user_index = 123;
        let user_keyhash =
            Hash256::from_str("244EA558C35EF9521EBA7418B72C94395235D678C6BDDD934EE514A6BC097FD8")
                .unwrap();
        let user_key = Key::Secp256r1(
            [
                2, 69, 17, 109, 179, 224, 216, 88, 134, 155, 142, 29, 222, 224, 160, 235, 116, 12,
                211, 16, 191, 65, 88, 180, 255, 202, 173, 80, 196, 146, 44, 111, 119,
            ]
            .into(),
        );

        let tx = r#"{
          "sender": "0xd7b73f486c66fa6daecd67d7aee46a26513b07c2",
          "credential": {
            "standard": {
              "signature": {
                "passkey": {
                  "sig": "L/ne0uoF3/aI73itjvcvW2AZ6fAJEd+QNSj/juzJc1zP9EeA++42ilmW03kJWlcqQKxTWaZQlEWCrdCnCnXU+A==",
                  "client_data": "eyJ0eXBlIjoid2ViYXV0aG4uZ2V0IiwiY2hhbGxlbmdlIjoiZ2lzQzkzblFTUWRzOVo2WEp6X0xEQXZOdHN0b3k2b091SERhMEl3ZllqcyIsIm9yaWdpbiI6Imh0dHA6Ly9sb2NhbGhvc3Q6NTA4MCIsImNyb3NzT3JpZ2luIjpmYWxzZX0=",
                  "authenticator_data": "SZYN5YgOjGh0NBcPZHZgW4/krrmihjLHmVzzuoMdl2MZAAAAAA=="
                }
              },
              "key_hash": "244EA558C35EF9521EBA7418B72C94395235D678C6BDDD934EE514A6BC097FD8"
            }
          },
          "data": {
            "chain_id": "dev-6",
            "user_index": 123,
            "nonce": 0
          },
          "msgs": [
            {
              "transfer": {
                "0x33361de42571d6aa20c37daa6da4b5ab67bfaad9": {
                  "bridge/usdc": "1000000"
                }
              }
            }
          ],
          "gas_limit": 2834
        }"#;

        let mut storage = MockStorage::new();

        account::STATUS
            .save(&mut storage, &AccountStatus::Active)
            .unwrap();

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
                    .insert(storage, (user_index, user_address))
                    .unwrap();
                account_factory::KEYS
                    .save(storage, (user_index, user_keyhash), &user_key)
                    .unwrap();
            });

        let mut ctx = MockContext::new()
            .with_storage(storage)
            .with_querier(querier)
            .with_contract(user_address)
            .with_chain_id("dev-6")
            .with_mode(AuthMode::Finalize);

        authenticate_tx(ctx.as_auth(), tx.deserialize_json().unwrap(), None).should_succeed();
    }

    #[test]
    fn eip712_authentication() {
        let user_address = Addr::from_str("0x9ee0274ae30d0e209bef2c7e6ce9675a92ef96c8").unwrap();
        let user_index = 123;
        let user_keyhash =
            Hash256::from_str("7D8FB7895BEAE0DF16E3E5F6FA7EB10CDE735E5B7C9A79DFCD8DD32A6BDD2165")
                .unwrap();
        let user_key =
            Key::Ethereum(Addr::from_str("0x4c9d879264227583f49af3c99eb396fe4735a935").unwrap());

        let mut storage = MockStorage::new();

        account::STATUS
            .save(&mut storage, &AccountStatus::Active)
            .unwrap();

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
                    .insert(storage, (user_index, user_address))
                    .unwrap();
                account_factory::KEYS
                    .save(storage, (user_index, user_keyhash), &user_key)
                    .unwrap();
            });

        let mut ctx = MockContext::new()
            .with_storage(storage)
            .with_querier(querier)
            .with_contract(user_address)
            .with_chain_id("dev-6")
            .with_mode(AuthMode::Finalize);

        let tx = r#"{
          "sender": "0x9ee0274ae30d0e209bef2c7e6ce9675a92ef96c8",
          "credential": {
            "standard": {
              "signature": {
                "eip712": {
                  "sig": "HVjOsIIx7o8SPbwXhSqpK/+N83V9Cz92+moM7vIIAaUz9YEelg8EesIN14ir5JcSSr/waG2b4gxPFbscaToUcBw=",
                  "typed_data": "eyJ0eXBlcyI6eyJFSVA3MTJEb21haW4iOlt7Im5hbWUiOiJuYW1lIiwidHlwZSI6InN0cmluZyJ9LHsibmFtZSI6ImNoYWluSWQiLCJ0eXBlIjoidWludDI1NiJ9LHsibmFtZSI6InZlcmlmeWluZ0NvbnRyYWN0IiwidHlwZSI6ImFkZHJlc3MifV0sIk1lc3NhZ2UiOlt7Im5hbWUiOiJzZW5kZXIiLCJ0eXBlIjoiYWRkcmVzcyJ9LHsibmFtZSI6ImRhdGEiLCJ0eXBlIjoiTWV0YWRhdGEifSx7Im5hbWUiOiJnYXNfbGltaXQiLCJ0eXBlIjoidWludDMyIn0seyJuYW1lIjoibWVzc2FnZXMiLCJ0eXBlIjoiVHhNZXNzYWdlW10ifV0sIk1ldGFkYXRhIjpbeyJuYW1lIjoidXNlcl9pbmRleCIsInR5cGUiOiJ1aW50MzIifSx7Im5hbWUiOiJjaGFpbl9pZCIsInR5cGUiOiJzdHJpbmcifSx7Im5hbWUiOiJub25jZSIsInR5cGUiOiJ1aW50MzIifV0sIlR4TWVzc2FnZSI6W3sibmFtZSI6InRyYW5zZmVyIiwidHlwZSI6IlRyYW5zZmVyIn1dLCJUcmFuc2ZlciI6W3sibmFtZSI6IjB4MzMzNjFkZTQyNTcxZDZhYTIwYzM3ZGFhNmRhNGI1YWI2N2JmYWFkOSIsInR5cGUiOiJDb2luMCJ9XSwiQ29pbjAiOlt7Im5hbWUiOiJicmlkZ2UvdXNkYyIsInR5cGUiOiJzdHJpbmcifV19LCJwcmltYXJ5VHlwZSI6Ik1lc3NhZ2UiLCJkb21haW4iOnsibmFtZSI6ImRhbmdvIiwiY2hhaW5JZCI6MSwidmVyaWZ5aW5nQ29udHJhY3QiOiIweDllZTAyNzRhZTMwZDBlMjA5YmVmMmM3ZTZjZTk2NzVhOTJlZjk2YzgifSwibWVzc2FnZSI6eyJzZW5kZXIiOiIweDllZTAyNzRhZTMwZDBlMjA5YmVmMmM3ZTZjZTk2NzVhOTJlZjk2YzgiLCJkYXRhIjp7ImNoYWluX2lkIjoiZGV2LTYiLCJ1c2VyX2luZGV4IjoxMjMsIm5vbmNlIjowfSwiZ2FzX2xpbWl0IjoyODM0LCJtZXNzYWdlcyI6W3sidHJhbnNmZXIiOnsiMHgzMzM2MWRlNDI1NzFkNmFhMjBjMzdkYWE2ZGE0YjVhYjY3YmZhYWQ5Ijp7ImJyaWRnZS91c2RjIjoiMTAwMDAwMCJ9fX1dfX0="
                }
              },
              "key_hash": "7D8FB7895BEAE0DF16E3E5F6FA7EB10CDE735E5B7C9A79DFCD8DD32A6BDD2165"
            }
          },
          "data": {
            "chain_id": "dev-6",
            "user_index": 123,
            "nonce": 0
          },
          "msgs": [
            {
              "transfer": {
                "0x33361de42571d6aa20c37daa6da4b5ab67bfaad9": {
                  "bridge/usdc": "1000000"
                }
              }
            }
          ],
          "gas_limit": 2834
        }"#;

        authenticate_tx(ctx.as_auth(), tx.deserialize_json::<Tx>().unwrap(), None).should_succeed();
    }

    #[test]
    fn secp256k1_authentication() {
        let user_address = addr!("843a9778a711d5474ef6efd65fca38731f281471");
        let user_index = 231893934;
        let user_keyhash =
            hash!("94eb754d36ed86af6fc231eae13c78b4a298ed065f63eb8dac139b8b943b76da");
        let user_key = Key::Secp256k1(
            hex!("022e730b2e26c6e3ce78d28c3700da0a798d893a73ee15055baaaee1cf46db7a4a").into(),
        );

        let tx = r#"{
          "sender": "0x843a9778a711d5474ef6efd65fca38731f281471",
          "gas_limit": 7378592558624777017,
          "msgs": [
            {
              "transfer": {
                "0x836ca678a5afe736c6b64b2d5a6ee4bc85588cd8": {
                  "bridge/usdc": "100000000"
                }
              }
            }
          ],
          "data": {
            "chain_id": "dev-1",
            "nonce": 27,
            "user_index": 231893934
          },
          "credential": {
            "standard": {
              "key_hash": "94EB754D36ED86AF6FC231EAE13C78B4A298ED065F63EB8DAC139B8B943B76DA",
              "signature": {
                "secp256k1": "LEohIzCuV3/MRLM/XvZNcxUdNp/Q811IsioZ3SEBbbRMvXlrvWi1v3+NYeZBYnALVtQzZcpO1E2wqiBd64lSdg=="
              }
            }
          }
        }"#;

        let mut storage = MockStorage::new();

        account::STATUS
            .save(&mut storage, &AccountStatus::Active)
            .unwrap();

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
                    .insert(storage, (user_index, user_address))
                    .unwrap();
                account_factory::KEYS
                    .save(storage, (user_index, user_keyhash), &user_key)
                    .unwrap();
            });

        // With account in the `Inactive` state. Should fail.
        let mut ctx = MockContext::new()
            .with_storage(MockStorage::default()) // use the default storage, which doesn't contain the active status flag
            .with_querier(querier)
            .with_contract(user_address)
            .with_chain_id("not-dev-1")
            .with_mode(AuthMode::Finalize);

        authenticate_tx(ctx.as_auth(), tx.deserialize_json().unwrap(), None)
            .should_fail_with_error("is not active");

        // With the incorrect chain ID. Should fail.
        let mut ctx = MockContext::new()
            .with_storage(storage) // use the storage that contains the active status flag
            .with_querier(ctx.querier)
            .with_contract(user_address)
            .with_chain_id("not-dev-1")
            .with_mode(AuthMode::Finalize);

        authenticate_tx(ctx.as_auth(), tx.deserialize_json().unwrap(), None)
            .should_fail_with_error("chain ID mismatch");

        // With the correct chain ID.
        let mut ctx = MockContext::new()
            .with_storage(ctx.storage)
            .with_querier(ctx.querier)
            .with_contract(user_address)
            .with_chain_id("dev-1")
            .with_mode(AuthMode::Finalize);

        authenticate_tx(ctx.as_auth(), tx.deserialize_json().unwrap(), None).should_succeed();
    }

    #[test]
    fn session_key_with_passkey_authentication() {
        let user_address = Addr::from_str("0xd7b73f486c66fa6daecd67d7aee46a26513b07c2").unwrap();
        let user_index = 123;
        let user_keyhash =
            Hash256::from_str("244EA558C35EF9521EBA7418B72C94395235D678C6BDDD934EE514A6BC097FD8")
                .unwrap();
        let user_key = Key::Secp256r1(
            [
                2, 69, 17, 109, 179, 224, 216, 88, 134, 155, 142, 29, 222, 224, 160, 235, 116, 12,
                211, 16, 191, 65, 88, 180, 255, 202, 173, 80, 196, 146, 44, 111, 119,
            ]
            .into(),
        );

        let mut storage = MockStorage::new();

        account::STATUS
            .save(&mut storage, &AccountStatus::Active)
            .unwrap();

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
                    .insert(storage, (user_index, user_address))
                    .unwrap();
                account_factory::KEYS
                    .save(storage, (user_index, user_keyhash), &user_key)
                    .unwrap();
            });

        let mut ctx = MockContext::new()
            .with_storage(storage)
            .with_querier(querier)
            .with_contract(user_address)
            .with_chain_id("dev-6")
            .with_mode(AuthMode::Finalize);

        let tx = r#"{
        "sender": "0xd7b73f486c66fa6daecd67d7aee46a26513b07c2",
        "credential": {
            "session": {
            "session_info": {
                "session_key": "AhpEarfTNt2uMf9jddpvEE6mqDl8J0yS2CJOFkQU96sV",
                "expire_at": "1764799201191"
            },
            "authorization": {
                "key_hash": "244EA558C35EF9521EBA7418B72C94395235D678C6BDDD934EE514A6BC097FD8",
                "signature": {
                "passkey": {
                    "sig": "CDy/TtzDxbAkMdHF1Zb1NwsRikbwDWt00Ck61zzDfwJnuAiYLVU6+JoSvGMgtqRxS4W0b551w1rC9kqSQ6mHXQ==",
                    "client_data": "eyJ0eXBlIjoid2ViYXV0aG4uZ2V0IiwiY2hhbGxlbmdlIjoiam9CdUxqUDhzU0pJcnh1ZDNkWllrTE5XSFhuN1lQaDI1WmxkZk1xRUktayIsIm9yaWdpbiI6Imh0dHA6Ly9sb2NhbGhvc3Q6NTA4MCIsImNyb3NzT3JpZ2luIjpmYWxzZX0=",
                    "authenticator_data": "SZYN5YgOjGh0NBcPZHZgW4/krrmihjLHmVzzuoMdl2MZAAAAAA=="
                }
                }
            },
            "session_signature": "3h/hnILoPCfyyC7+qEWuku2tePMLrI7ksHI3D9hEIMFOPp9FAQB7coCmd2k12D8P7RrTOznNsDgVd2dkMebehQ=="
            }
        },
        "data": {
            "chain_id": "dev-6",
            "user_index": 123,
            "nonce": 0
        },
        "msgs": [
            {
            "transfer": {
                "0x33361de42571d6aa20c37daa6da4b5ab67bfaad9": {
                "bridge/usdc": "1000000"
                }
            }
            }
        ],
        "gas_limit": 2834
        }"#;

        authenticate_tx(ctx.as_auth(), tx.deserialize_json::<Tx>().unwrap(), None).should_succeed();
    }

    #[test]
    fn session_key_with_eip712_authentication() {
        let user_address = Addr::from_str("0x9ee0274ae30d0e209bef2c7e6ce9675a92ef96c8").unwrap();
        let user_index = 123;
        let user_keyhash =
            Hash256::from_str("7D8FB7895BEAE0DF16E3E5F6FA7EB10CDE735E5B7C9A79DFCD8DD32A6BDD2165")
                .unwrap();
        let user_key =
            Key::Ethereum(Addr::from_str("0x4c9d879264227583f49af3c99eb396fe4735a935").unwrap());

        let mut storage = MockStorage::new();

        account::STATUS
            .save(&mut storage, &AccountStatus::Active)
            .unwrap();

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
                    .insert(storage, (user_index, user_address))
                    .unwrap();
                account_factory::KEYS
                    .save(storage, (user_index, user_keyhash), &user_key)
                    .unwrap();
            });

        let mut ctx = MockContext::new()
            .with_storage(storage)
            .with_querier(querier)
            .with_contract(user_address)
            .with_chain_id("dev-6")
            .with_mode(AuthMode::Finalize);

        let tx = r#"{
          "sender": "0x9ee0274ae30d0e209bef2c7e6ce9675a92ef96c8",
          "credential": {
            "session": {
              "session_info": {
                "session_key": "A/+qq0IZ/nVmajLRoA3RBjKaWMDQ2TVnvGNZvPSFyf4m",
                "expire_at": "1764699053993"
              },
              "authorization": {
                "key_hash": "7D8FB7895BEAE0DF16E3E5F6FA7EB10CDE735E5B7C9A79DFCD8DD32A6BDD2165",
                "signature": {
                  "eip712": {
                    "sig": "W9Wf2kO1RWcECwyf1vsZLbRLN9JZoMN/ca3DikEAtsI8+bWI3YCMRvhrJ18SOJ2K7B2pL5wjVBf/+EdUs5xRHBs=",
                    "typed_data": "eyJkb21haW4iOnsibmFtZSI6IkRhbmdvQXJiaXRyYXJ5TWVzc2FnZSIsImNoYWluSWQiOjEsInZlcmlmeWluZ0NvbnRyYWN0IjoiMHgwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwIn0sIm1lc3NhZ2UiOnsic2Vzc2lvbl9rZXkiOiJBLytxcTBJWi9uVm1hakxSb0EzUkJqS2FXTURRMlRWbnZHTlp2UFNGeWY0bSIsImV4cGlyZV9hdCI6IjE3NjQ2OTkwNTM5OTMifSwicHJpbWFyeVR5cGUiOiJNZXNzYWdlIiwidHlwZXMiOnsiRUlQNzEyRG9tYWluIjpbeyJuYW1lIjoibmFtZSIsInR5cGUiOiJzdHJpbmcifSx7Im5hbWUiOiJjaGFpbklkIiwidHlwZSI6InVpbnQyNTYifSx7Im5hbWUiOiJ2ZXJpZnlpbmdDb250cmFjdCIsInR5cGUiOiJhZGRyZXNzIn1dLCJNZXNzYWdlIjpbeyJuYW1lIjoic2Vzc2lvbl9rZXkiLCJ0eXBlIjoic3RyaW5nIn0seyJuYW1lIjoiZXhwaXJlX2F0IiwidHlwZSI6InN0cmluZyJ9XX19"
                  }
                }
              },
              "session_signature": "X9Npvv1WnUEbeTtrug5Zz6WSG8K2h73B/df3GHe6rkJ5UifGLK2oanHidKYyNeNY3OXxQjVzpvDRT7LbkAmczA=="
            }
          },
          "data": {
            "chain_id": "dev-6",
            "user_index": 123,
            "nonce": 0
          },
          "msgs": [
            {
              "transfer": {
                "0x33361de42571d6aa20c37daa6da4b5ab67bfaad9": {
                  "bridge/usdc": "1000000"
                }
              }
            }
          ],
          "gas_limit": 2834
        }"#;

        authenticate_tx(ctx.as_auth(), tx.deserialize_json::<Tx>().unwrap(), None).should_succeed();
    }

    #[test]
    fn session_key_with_secp256k1_authentication() {
        let user_address = addr!("9117495f17163ec82e4ae424b7f2227dd21d3ce5");
        let user_index = 1733837080;
        let user_keyhash =
            hash!("3378fadf5422e7e1cbe68fcac26e355238c437dc36139212bcdfe6fe00e4e96f");
        let user_key = Key::Secp256k1(
            hex!("035d1e23762e9436aaff9fd41322cf7ea3d6a5a282094f85d175035ae9ca1ea265").into(),
        );

        let mut storage = MockStorage::new();

        account::STATUS
            .save(&mut storage, &AccountStatus::Active)
            .unwrap();

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
                    .insert(storage, (user_index, user_address))
                    .unwrap();
                account_factory::KEYS
                    .save(storage, (user_index, user_keyhash), &user_key)
                    .unwrap();
            });

        let mut ctx = MockContext::new()
            .with_storage(storage)
            .with_querier(querier)
            .with_contract(user_address)
            .with_chain_id("dev-1")
            .with_mode(AuthMode::Finalize);

        let tx = r#"{
          "sender": "0x9117495f17163ec82e4ae424b7f2227dd21d3ce5",
          "gas_limit": 6348334294010820860,
          "msgs": [
            {
              "transfer": {
                "0xe2a560440d34e43c1c02d7ce4f2ed2e86fa3367d": {
                  "bridge/usdc": "100000000"
                }
              }
            }
          ],
          "data": {
            "chain_id": "dev-1",
            "nonce": 44,
            "user_index": 1733837080
          },
          "credential": {
            "session": {
              "authorization": {
                "key_hash": "3378FADF5422E7E1CBE68FCAC26E355238C437DC36139212BCDFE6FE00E4E96F",
                "signature": {
                  "secp256k1": "U0MuCfpy8xuLKFlpvP4byrSLUvRuf5QWBaVfSRd+KnEtKQmU+4zvVyWFRf9KFiq2oUObN8LuG3cY0TQIeNuSHQ=="
                }
              },
              "session_info": {
                "expire_at": "340282366920938463463374607431.768211455",
                "session_key": "Ax614yDvhtfapEA66dAM0QH6ZrXqwSWX3Xxy+mikaPgC"
              },
              "session_signature": "HnhRpEQXcltog6DFNKq3u2wIjoYpOkjfeTMFvPmP2Styl6f8IZfHeOtGZgLgBj5U6NH1PObP6SRkVaPZE7hMgg=="
            }
          }
        }"#;

        authenticate_tx(ctx.as_auth(), tx.deserialize_json::<Tx>().unwrap(), None).should_succeed();
    }

    #[test]
    fn authenticate_onboarding_eip712() {
        let user_key =
            Key::Ethereum(Addr::from_str("0x4c9d879264227583f49af3c99eb396fe4735a935").unwrap());

        let mut storage = MockStorage::new();

        account::STATUS
            .save(&mut storage, &AccountStatus::Active)
            .unwrap();

        let ctx = MockContext::new()
            .with_storage(storage)
            .with_chain_id("dev-6")
            .with_mode(AuthMode::Finalize);

        let signature = r#"{
          "eip712": {
            "sig": "zHPA/hLrFD3BSaFBbbdygCwCsII6mVHbIRSQYpNcN01pL81Gi2n8xNe0OTinG749kZjq/xGZYcHdlKRq3YwDBRs=",
            "typed_data": "eyJkb21haW4iOnsibmFtZSI6IkRhbmdvQXJiaXRyYXJ5TWVzc2FnZSIsImNoYWluSWQiOjEsInZlcmlmeWluZ0NvbnRyYWN0IjoiMHgwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwIn0sIm1lc3NhZ2UiOnsiY2hhaW5faWQiOiJkZXYtNiJ9LCJwcmltYXJ5VHlwZSI6Ik1lc3NhZ2UiLCJ0eXBlcyI6eyJFSVA3MTJEb21haW4iOlt7Im5hbWUiOiJuYW1lIiwidHlwZSI6InN0cmluZyJ9LHsibmFtZSI6ImNoYWluSWQiLCJ0eXBlIjoidWludDI1NiJ9LHsibmFtZSI6InZlcmlmeWluZ0NvbnRyYWN0IiwidHlwZSI6ImFkZHJlc3MifV0sIk1lc3NhZ2UiOlt7Im5hbWUiOiJjaGFpbl9pZCIsInR5cGUiOiJzdHJpbmcifV19fQ=="
          }
        }"#.deserialize_json::<Signature>().unwrap();

        verify_signature(
            &ctx.api,
            user_key,
            signature,
            VerifyData::Onboard(RegisterUserData {
                chain_id: "dev-6".into(),
            }),
        )
        .should_succeed();
    }
}
