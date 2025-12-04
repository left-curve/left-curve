use {
    dango_auth::MAX_NONCE_INCREASE,
    dango_types::auth::{
        Credential, Metadata, SessionCredential, SessionInfo, SignDoc, Signature,
        StandardCredential,
    },
    grug::{
        Addr, ByteArray, Hash256, HashExt, Inner, JsonSerExt, MOCK_CHAIN_ID, Message, NonEmpty,
        SignData, Timestamp, Tx, coins,
    },
    identity::Identity256,
    k256::ecdsa::signature::DigestSigner,
    rand::{Rng, RngCore},
};

fn generate_secp256k1_standard_test_data() -> anyhow::Result<()> {
    let (sk, vk, vk_hash) = generate_random_secp256k1_key_pair()?;
    let sign_doc = generate_random_unsigned_transaction()?;

    let credential = {
        let signature = secp256k1_sign(&sk, &sign_doc)?;
        Credential::Standard(StandardCredential {
            key_hash: vk_hash,
            signature: Signature::Secp256k1(signature),
        })
    };

    let tx = Tx {
        sender: sign_doc.sender,
        gas_limit: sign_doc.gas_limit,
        msgs: sign_doc.messages,
        data: sign_doc.data.to_json_value()?,
        credential: credential.to_json_value()?,
    };

    println!("user_address = {}", sign_doc.sender);
    println!("user_index   = {}", sign_doc.data.user_index);
    println!("user_keyhash = {}", hex::encode(vk_hash));
    println!("user_key     = {}", hex::encode(vk));
    println!("tx:\n{}", tx.to_json_string_pretty()?);

    Ok(())
}

fn generate_secp256k1_session_test_data() -> anyhow::Result<()> {
    // 1 is the main key, 2 is the session key.
    let (sk1, vk1, vk1_hash) = generate_random_secp256k1_key_pair()?;
    let (sk2, vk2, _) = generate_random_secp256k1_key_pair()?;
    let sign_doc = generate_random_unsigned_transaction()?;

    let session_info = SessionInfo {
        session_key: vk2.into(),
        expire_at: Timestamp::from_nanos(u128::MAX),
    };

    // Main key signs the authorization.
    let authorization = {
        let signature = secp256k1_sign(&sk1, &session_info)?;
        StandardCredential {
            key_hash: vk1_hash,
            signature: Signature::Secp256k1(signature),
        }
    };

    // Session keys signs the transaction.
    let session_signature = secp256k1_sign(&sk2, &sign_doc)?;

    let credential = Credential::Session(SessionCredential {
        session_info,
        session_signature,
        authorization,
    });

    let tx = Tx {
        sender: sign_doc.sender,
        gas_limit: sign_doc.gas_limit,
        msgs: sign_doc.messages,
        data: sign_doc.data.to_json_value()?,
        credential: credential.to_json_value()?,
    };

    println!("user_address = {}", sign_doc.sender);
    println!("user_index   = {}", sign_doc.data.user_index);
    println!("user_keyhash = {}", hex::encode(vk1_hash));
    println!("user_key     = {}", hex::encode(vk1));
    println!("tx:\n{}", tx.to_json_string_pretty()?);

    Ok(())
}

fn generate_random_unsigned_transaction() -> anyhow::Result<SignDoc> {
    let mut sender = Addr::mock(0);
    rand::thread_rng().fill_bytes(&mut sender);

    let mut recipient = Addr::mock(0);
    rand::thread_rng().fill_bytes(&mut recipient);

    let user_index = rand::thread_rng().r#gen();
    let nonce = rand::thread_rng().gen_range(0..MAX_NONCE_INCREASE);
    let gas_limit = rand::thread_rng().r#gen();

    let messages = NonEmpty::new_unchecked(vec![Message::transfer(
        recipient,
        coins! { "bridge/usdc" => 100_000_000 },
    )?]);

    let data = Metadata {
        chain_id: MOCK_CHAIN_ID.to_string(),
        user_index,
        nonce,
        expiry: None,
    };

    Ok(SignDoc {
        sender,
        gas_limit,
        messages,
        data,
    })
}

/// Return the private key, public key, and SHA-256 hash of the public key.
fn generate_random_secp256k1_key_pair()
-> anyhow::Result<(k256::ecdsa::SigningKey, [u8; 33], Hash256)> {
    let sk = k256::ecdsa::SigningKey::random(&mut rand::rngs::OsRng);
    let vk: [u8; 33] = sk
        .verifying_key()
        .to_encoded_point(true)
        .as_bytes()
        .try_into()?;
    let vk_hash = vk.hash256();

    Ok((sk, vk, vk_hash))
}

fn secp256k1_sign<T>(sk: &k256::ecdsa::SigningKey, sign_doc: &T) -> anyhow::Result<ByteArray<64>>
where
    T: SignData,
    anyhow::Error: From<T::Error>,
{
    let prehash_sign_data = sign_doc.to_prehash_sign_data()?;
    let sign_data = prehash_sign_data.hash256();
    let digest = Identity256::from(sign_data.into_inner());
    let signature: k256::ecdsa::Signature = sk.sign_digest(digest);

    Ok(ByteArray::from_inner(signature.to_bytes().into()))
}

fn main() -> anyhow::Result<()> {
    println!("===================== Secp256k1 Standard ======================");
    generate_secp256k1_standard_test_data()?;

    println!("====================== Secp256k1 Session ======================");
    generate_secp256k1_session_test_data()?;

    Ok(())
}
