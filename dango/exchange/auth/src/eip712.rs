use {
    crate::EIP155_CHAIN_ID,
    alloy::{
        dyn_abi::{Eip712Domain, Resolver, TypedData},
        primitives::{Address, U160, address},
    },
    dango_primitives::{Inner, Json, JsonDeExt, JsonSerExt, Message, json},
    dango_types::{
        account_factory::RegisterUserData,
        auth::{SessionInfo, SignDoc},
    },
};

const DOMAIN_NAME: &str = "dango";
const PRIMARY_TYPE: &str = "Message";
const ZERO_ADDR: Address = address!("0x0000000000000000000000000000000000000000");

pub fn typed_data_for_transaction(sign_doc: &SignDoc) -> anyhow::Result<TypedData> {
    let verifying_contract = U160::from_be_bytes(sign_doc.sender.into_inner()).into();
    Ok(TypedData {
        resolver: transaction_resolver(sign_doc.data.expiry.is_some())?,
        domain: dango_domain(Some(verifying_contract)),
        primary_type: PRIMARY_TYPE.to_string(),
        message: transaction_message(sign_doc)?.into_inner(),
    })
}

pub fn typed_data_for_session(session_info: &SessionInfo) -> anyhow::Result<TypedData> {
    Ok(TypedData {
        resolver: session_resolver()?,
        domain: dango_domain(Some(ZERO_ADDR)),
        primary_type: PRIMARY_TYPE.to_string(),
        message: session_message(session_info)?.into_inner(),
    })
}

pub fn typed_data_for_onboard(data: &RegisterUserData) -> anyhow::Result<TypedData> {
    Ok(TypedData {
        resolver: onboard_resolver(data.referrer.is_some())?,
        domain: dango_domain(Some(ZERO_ADDR)),
        primary_type: PRIMARY_TYPE.to_string(),
        message: onboard_message(data)?.into_inner(),
    })
}

// ----------------------------- resolvers -----------------------------

fn transaction_resolver(has_expiry: bool) -> anyhow::Result<Resolver> {
    // `gas_limit` is `string` rather than `uint64` so the JSON value is also
    // a string, avoiding JS `Number` precision loss for values above 2^53.
    json!({
        "Message": [
            { "name": "sender",    "type": "address"      },
            { "name": "data",      "type": "Metadata"     },
            { "name": "gas_limit", "type": "string"       },
            { "name": "messages",  "type": "TxMessage[]"  },
        ],
        "Metadata": metadata_fields(has_expiry),
        "TxMessage": [
            { "name": "kind",    "type": "string" },
            { "name": "payload", "type": "string" },
        ],
    })
    .deserialize_json()
    .map_err(Into::into)
}

fn session_resolver() -> anyhow::Result<Resolver> {
    // `session_key` is `string` rather than `bytes` because `ByteArray<33>`
    // serializes to base64, while EIP-712 `bytes` expects hex.
    json!({
        "Message": [
            { "name": "chain_id",    "type": "string" },
            { "name": "session_key", "type": "string" },
            { "name": "expire_at",   "type": "string" },
        ],
    })
    .deserialize_json()
    .map_err(Into::into)
}

fn onboard_resolver(has_referrer: bool) -> anyhow::Result<Resolver> {
    // `key_hash` is `string` rather than `bytes32` because `Hash256`
    // serializes to unprefixed uppercase hex, while EIP-712 `bytes32`
    // expects `0x`-prefixed lowercase hex.
    let mut fields = vec![
        json!({ "name": "chain_id", "type": "string" }),
        json!({ "name": "key",      "type": "string" }),
        json!({ "name": "key_hash", "type": "string" }),
        json!({ "name": "seed",     "type": "uint32" }),
    ];
    if has_referrer {
        fields.push(json!({ "name": "referrer", "type": "uint32" }));
    }
    json!({ "Message": fields })
        .deserialize_json()
        .map_err(Into::into)
}

fn metadata_fields(has_expiry: bool) -> Vec<Json> {
    let mut fields = vec![
        json!({ "name": "user_index", "type": "uint32" }),
        json!({ "name": "chain_id",   "type": "string" }),
        json!({ "name": "nonce",      "type": "uint32" }),
    ];
    if has_expiry {
        fields.push(json!({ "name": "expiry", "type": "string" }));
    }
    fields
}

// ------------------------------ messages ------------------------------

fn transaction_message(sign_doc: &SignDoc) -> anyhow::Result<Json> {
    let messages = sign_doc
        .messages
        .iter()
        .map(tx_message_entry)
        .collect::<anyhow::Result<Vec<Json>>>()?;

    let mut data = json!({
        "user_index": sign_doc.data.user_index,
        "chain_id":   sign_doc.data.chain_id,
        "nonce":      sign_doc.data.nonce,
    });
    if let Some(expiry) = sign_doc.data.expiry {
        data["expiry"] = expiry.to_json_value()?.into_inner();
    }

    Ok(json!({
        "sender":    sign_doc.sender,
        "data":      data,
        "gas_limit": sign_doc.gas_limit.to_string(),
        "messages":  messages,
    }))
}

fn session_message(session_info: &SessionInfo) -> anyhow::Result<Json> {
    Ok(json!({
        "chain_id":    session_info.chain_id,
        "session_key": session_info.session_key,
        "expire_at":   session_info.expire_at,
    }))
}

fn onboard_message(data: &RegisterUserData) -> anyhow::Result<Json> {
    let mut message = json!({
        "chain_id": data.chain_id,
        "key":      canonical_payload(&data.key)?,
        "key_hash": data.key_hash,
        "seed":     data.seed,
    });
    if let Some(referrer) = data.referrer {
        message["referrer"] = referrer.to_json_value()?.into_inner();
    }
    Ok(message)
}

fn tx_message_entry(msg: &Message) -> anyhow::Result<Json> {
    let (kind, payload) = match msg {
        Message::Configure(m) => ("configure", canonical_payload(m)?),
        Message::Upgrade(m) => ("upgrade", canonical_payload(m)?),
        Message::Transfer(m) => ("transfer", canonical_payload(m)?),
        Message::Upload(m) => ("upload", canonical_payload(m)?),
        Message::Instantiate(m) => ("instantiate", canonical_payload(m)?),
        Message::Execute(m) => ("execute", canonical_payload(m)?),
        Message::Migrate(m) => ("migrate", canonical_payload(m)?),
    };
    Ok(json!({ "kind": kind, "payload": payload }))
}

// Route through `to_json_value` so the result has every object key sorted
// alphabetically (see `JsonSerExt::to_json_value`). Serializing through
// `to_json_vec` directly would emit struct fields in declared order, which
// would diverge from the TypeScript signer.
fn canonical_payload<T>(value: &T) -> anyhow::Result<String>
where
    T: JsonSerExt,
{
    Ok(value.to_json_value()?.to_json_string()?)
}

fn dango_domain(verifying_contract: Option<Address>) -> Eip712Domain {
    Eip712Domain {
        name: Some(DOMAIN_NAME.into()),
        chain_id: Some(EIP155_CHAIN_ID),
        verifying_contract,
        ..Default::default()
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    //! Cross-language fixtures. The TypeScript signer must compute identical
    //! EIP-712 digests for the same inputs, so these tests double as
    //! reference vectors for `sdk/typescript/utils/src/typedData.spec.ts`.

    use {
        super::*,
        dango_primitives::{Addr, ByteArray, Coins, HashExt, MsgExecute, NonEmpty, Timestamp},
        dango_types::{
            account_factory::RegisterUserData,
            auth::{Key, Metadata},
        },
        hex_literal::hex,
        std::{collections::BTreeMap, str::FromStr},
    };

    fn fixture_sender() -> Addr {
        Addr::from_str("0x1234567890123456789012345678901234567890").unwrap()
    }

    fn fixture_recipient() -> Addr {
        Addr::from_str("0xabcdefabcdefabcdefabcdefabcdefabcdefabcd").unwrap()
    }

    /// Transaction with a `Transfer` and an `Execute` message; `gas_limit`
    /// above `2^53` to exercise the string encoding; declared-order field
    /// names inside `MsgExecute` differ from alphabetical to exercise the
    /// canonical sort.
    fn fixture_sign_doc() -> SignDoc {
        let mut transfer = BTreeMap::new();
        transfer.insert(
            fixture_recipient(),
            Coins::one("usdc", 1_000_000u128).unwrap(),
        );

        let execute = MsgExecute {
            contract: fixture_recipient(),
            msg: json!({ "foo": "bar" }),
            funds: Coins::new(),
        };

        SignDoc {
            sender: fixture_sender(),
            gas_limit: 16_075_769_052_062_025_908,
            messages: NonEmpty::new_unchecked(vec![
                Message::Transfer(transfer),
                Message::Execute(execute),
            ]),
            data: Metadata {
                user_index: 42,
                chain_id: "dev-1".to_string(),
                nonce: 5,
                expiry: None,
            },
        }
    }

    fn fixture_session_info() -> SessionInfo {
        SessionInfo {
            chain_id: "dev-1".to_string(),
            session_key: ByteArray::from_inner([0x02; 33]),
            expire_at: Timestamp::from_nanos(1_700_000_000_000_000_000),
        }
    }

    fn fixture_register_data() -> RegisterUserData {
        let key_addr = Addr::from_str("0x1111111111111111111111111111111111111111").unwrap();
        RegisterUserData {
            chain_id: "dev-1".to_string(),
            key: Key::Ethereum(key_addr),
            key_hash: key_addr.as_ref().hash256(),
            seed: 7,
            referrer: None,
        }
    }

    #[test]
    fn transaction_digest_is_stable() {
        let typed_data = typed_data_for_transaction(&fixture_sign_doc()).unwrap();
        let digest = typed_data.eip712_signing_hash().unwrap();
        assert_eq!(
            digest.0,
            hex!("e2ec813e42e60bdc53b296440153dc9128b9317025f2c280d0ae90f57681550c"),
            "transaction digest changed; if intentional, update this fixture AND the matching TS test"
        );
    }

    #[test]
    fn session_digest_is_stable() {
        let typed_data = typed_data_for_session(&fixture_session_info()).unwrap();
        let digest = typed_data.eip712_signing_hash().unwrap();
        assert_eq!(
            digest.0,
            hex!("4adf7aa1c10e6d0f080afdbce4f032fca952b73a5d9cf22a56ae77f30c35d297"),
            "session digest changed; if intentional, update this fixture AND the matching TS test"
        );
    }

    #[test]
    fn onboard_digest_is_stable() {
        let typed_data = typed_data_for_onboard(&fixture_register_data()).unwrap();
        let digest = typed_data.eip712_signing_hash().unwrap();
        assert_eq!(
            digest.0,
            hex!("aab3d150835e6fb79fb2158db3848ab78dd6cf7afb8d08209fd6003392421731"),
            "onboard digest changed; if intentional, update this fixture AND the matching TS test"
        );
    }

    #[test]
    fn execute_payload_keys_are_sorted() {
        // `MsgExecute` declares `contract, msg, funds`. The canonical payload
        // must emit them in alphabetical order: `contract, funds, msg`.
        let payload = canonical_payload(&MsgExecute {
            contract: fixture_recipient(),
            msg: json!({ "z": 1, "a": 2 }),
            funds: Coins::new(),
        })
        .unwrap();
        let contract_pos = payload.find("contract").unwrap();
        let funds_pos = payload.find("funds").unwrap();
        let msg_pos = payload.find("\"msg\":").unwrap();
        assert!(contract_pos < funds_pos);
        assert!(funds_pos < msg_pos);
    }
}
