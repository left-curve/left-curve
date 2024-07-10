use {
    crate::{AccountData, PublicKey, TxOrder, PUBLIC_KEY, SEQUENCE, UNORDERED_TXS},
    anyhow::ensure,
    grug_storage::Bound,
    grug_types::{
        from_json_key_value, to_json_vec, Addr, Attribute, AuthCtx, Hash, Message, MutableCtx,
        Order, Response, StdError, StdResult, Storage, Timestamp, Tx,
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
/// for preventing replay attacks (e.g. user signs a transaction for chain A;
/// attacker uses the signature to broadcast another transaction on chain B.)
pub fn make_sign_bytes<Hasher, const HASH_LEN: usize>(
    hasher: Hasher,
    msgs: &[Message],
    sender: &Addr,
    chain_id: &str,
    sequence: Option<u32>,
    expiration_timestamp: Option<u128>,
) -> StdResult<[u8; HASH_LEN]>
where
    Hasher: Fn(&[u8]) -> [u8; HASH_LEN],
{
    let mut prehash = Vec::new();
    // That there are multiple valid ways that the messages can be serialized
    // into JSON. Here we use `grug::to_json_vec` as the source of truth.
    prehash.extend(to_json_vec(&msgs)?);
    prehash.extend(sender.as_ref());
    prehash.extend(chain_id.as_bytes());
    if let Some(sequence) = sequence {
        prehash.extend(sequence.to_be_bytes());
    }
    if let Some(expiration_timestamp) = expiration_timestamp {
        prehash.extend(expiration_timestamp.to_be_bytes());
    }

    Ok(hasher(&prehash))
}

pub fn initialize(storage: &mut dyn Storage, public_key: &PublicKey) -> StdResult<Response> {
    // Save the public key in contract store
    PUBLIC_KEY.save(storage, public_key)?;

    // Initialize the sequence number to zero
    SEQUENCE.initialize(storage)?;

    Ok(Response::new())
}

pub fn update_key(ctx: MutableCtx, new_public_key: &PublicKey) -> anyhow::Result<Response> {
    // Only the account itself can update its key
    ensure!(ctx.sender == ctx.contract, "Nice try lol");

    // Save the new public key
    PUBLIC_KEY.save(ctx.storage, new_public_key)?;

    Ok(Response::new())
}

pub fn authenticate_tx(ctx: AuthCtx, tx: Tx) -> StdResult<Response> {
    let remove_attributes = remove_expired_unordered_txs(ctx.storage, ctx.block.timestamp)?;

    let account_data: AccountData = from_json_key_value(tx.data, "account")?;

    let (hash, attributes) = match account_data.order {
        TxOrder::Ordered => {
            let sequence = SEQUENCE.load(ctx.storage)?;

            // Increment the sequence number
            SEQUENCE.increment(ctx.storage)?;

            // Prepare the hash that is expected to have been signed
            let hash = make_sign_bytes(
                // Note: We can't use a trait method as a function pointer. Need to use
                // a closure instead.
                |prehash| ctx.api.sha2_256(prehash),
                &tx.msgs,
                &tx.sender,
                &ctx.chain_id,
                Some(sequence),
                None,
            )?;

            let attributes = vec![Attribute::new("key", sequence.to_string())];

            (hash, attributes)
        },
        TxOrder::Unordered { expiration } => {
            if expiration < ctx.block.timestamp {
                return Err(StdError::generic_err("Transaction expired"));
            }

            let expiration = expiration.nanos();
            // Prepare the hash that is expected to have been signed
            let hash = make_sign_bytes(
                // Note: We can't use a trait method as a function pointer. Need to use
                // a closure instead.
                |prehash| ctx.api.sha2_256(prehash),
                &tx.msgs,
                &tx.sender,
                &ctx.chain_id,
                None,
                Some(expiration),
            )?;

            if UNORDERED_TXS.has(ctx.storage, (expiration, &Hash::from_slice(hash))) {
                return Err(StdError::generic_err("Transaction already exists"));
            } else {
                UNORDERED_TXS.insert(ctx.storage, (expiration, &Hash::from_slice(hash)))?;
            }

            (hash, vec![])
        },
    };

    let public_key = PUBLIC_KEY.load(ctx.storage)?;

    // Verify the signature
    // Skip if we are in simulate mode
    if !ctx.simulate {
        match &public_key {
            PublicKey::Secp256k1(bytes) => {
                ctx.api.secp256k1_verify(&hash, &tx.credential, bytes)?;
            },
            PublicKey::Secp256r1(bytes) => {
                ctx.api.secp256r1_verify(&hash, &tx.credential, bytes)?;
            },
        }
    }

    Ok(Response::new()
        .add_attribute("method", "before_tx")
        .add_attributes(remove_attributes)
        .add_attributes(attributes))
}

fn remove_expired_unordered_txs(
    storage: &mut dyn Storage,
    current_timestamp: Timestamp,
) -> StdResult<Vec<Attribute>> {
    let to_remove: Vec<(u128, Hash)> = UNORDERED_TXS
        .range(
            storage,
            None,
            Some(Bound::exclusive((current_timestamp.nanos(), &Hash::ZERO))),
            Order::Ascending,
        )
        .collect::<StdResult<_>>()?;

    let attributes = to_remove
        .into_iter()
        .map(|(timestamp, hash)| {
            UNORDERED_TXS.remove(storage, (timestamp, &hash));
            Attribute::new(
                "unordered_expired",
                format!("timestamp: {} - bytes: {}", timestamp, hash),
            )
        })
        .collect();

    Ok(attributes)
}

#[cfg(test)]
mod tests {
    use {
        crate::{
            authenticate_tx, initialize, make_sign_bytes, AccountData, PublicKey, TxOrder,
            DATA_ACCOUNT_KEY,
        },
        grug_crypto::{sha2_256, Identity256},
        grug_types::{
            Addr, AuthCtx, BlockInfo, DataBuilder, Hash, Message, MockApi, MockStorage, Querier,
            QuerierWrapper, QueryRequest, QueryResponse, StdResult, Timestamp, Tx,
        },
        k256::ecdsa::{signature::DigestSigner, Signature, SigningKey, VerifyingKey},
        rand::rngs::OsRng,
        std::{borrow::BorrowMut, fmt::Display, ops::Deref},
    };

    trait UnwrapError {
        type Error;
        fn unwrap_err_contains(self, text: impl Into<String>) -> Self::Error;
    }

    impl<T, E: Display> UnwrapError for Result<T, E> {
        type Error = E;

        fn unwrap_err_contains(self, text: impl Into<String>) -> Self::Error {
            let text: String = text.into();

            let err = match self {
                Ok(_) => panic!("Result is not error, error {text} not found"),
                Err(e) => e,
            };

            if format!("{:#}", err).contains(&text) {
                err
            } else {
                panic!("{text} not contained in {err:#}")
            }
        }
    }
    struct MockQuerier;

    impl Querier for MockQuerier {
        fn query_chain(&self, _req: QueryRequest) -> StdResult<QueryResponse> {
            unimplemented!()
        }
    }

    fn build_tx_and_sign(
        sk: &SigningKey,
        msgs: Vec<Message>,
        sender: &Addr,
        sequence: Option<u32>,
        expiration_timestamp: Option<Timestamp>,
    ) -> Tx {
        let msg_hash = make_sign_bytes(
            sha2_256,
            &msgs,
            sender,
            "grug-1",
            sequence,
            expiration_timestamp.map(|val| val.nanos()),
        )
        .unwrap();

        let digest = Identity256::from(msg_hash);

        let sig: Signature = sk.sign_digest(digest);

        Tx {
            sender: sender.clone(),
            msgs,
            data: DataBuilder::default()
                .add_field(DATA_ACCOUNT_KEY, AccountData {
                    order: match (sequence, expiration_timestamp) {
                        (Some(_), None) => TxOrder::Ordered,
                        (None, Some(expiration)) => TxOrder::Unordered { expiration },
                        _ => panic!("Invalid combination of sequence and expiration"),
                    },
                })
                .unwrap()
                .finalize(),
            credential: sig.to_bytes().to_vec().into(),
            gas_limit: 0,
        }
    }

    #[test]
    fn unordered_tx() {
        let mut storage = MockStorage::new();

        let sk = SigningKey::random(&mut OsRng);
        let vk = VerifyingKey::from(&sk);

        let sender = Addr::mock(2);

        let mut ctx = AuthCtx {
            storage: storage.borrow_mut(),
            api: &MockApi,
            querier: QuerierWrapper::new(&MockQuerier),
            chain_id: "grug-1".to_string(),
            block: BlockInfo {
                height: 10_u64.into(),
                timestamp: Timestamp::from_nanos(100),
                hash: Hash::ZERO,
            },
            contract: Addr::mock(1),
            simulate: false,
        };

        // Initialize the contract

        initialize(
            ctx.storage,
            &PublicKey::Secp256k1(vk.to_sec1_bytes().deref().into()),
        )
        .unwrap();

        // create an unordered tx

        // Invalid expiration timestamp
        let tx = build_tx_and_sign(&sk, vec![], &sender, None, Some(Timestamp::from_nanos(8)));
        authenticate_tx(ctx.branch(), tx).unwrap_err_contains("Transaction expired");

        // Valid expiration timestamp
        let tx = build_tx_and_sign(&sk, vec![], &sender, None, Some(Timestamp::from_nanos(200)));
        authenticate_tx(ctx.branch(), tx.clone()).unwrap();

        // increace block time
        ctx.block.timestamp = Timestamp::from_nanos(200);

        // This should fail as the transaction still in the store
        authenticate_tx(ctx.branch(), tx.clone()).unwrap_err_contains("Transaction already exists");

        // increace block time
        ctx.block.timestamp = Timestamp::from_nanos(201);

        // This should fail as the transaction is alredy expired
        authenticate_tx(ctx.branch(), tx.clone()).unwrap_err_contains("Transaction expired");

        let tx = build_tx_and_sign(&sk, vec![], &sender, None, Some(Timestamp::from_nanos(300)));

        authenticate_tx(ctx.branch(), tx.clone()).unwrap();
    }
}
