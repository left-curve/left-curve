use {
    grug_account::Credential,
    grug_crypto::{sha2_256, Identity256},
    grug_types::{to_json_value, Addr, Binary, Hash256, Message, Tx, GENESIS_SENDER},
    k256::ecdsa::{signature::DigestSigner, Signature, SigningKey},
    rand::rngs::OsRng,
    std::collections::HashMap,
};

pub type TestAccounts = HashMap<&'static str, TestAccount>;

pub struct TestAccount {
    pub address: Addr,
    pub sk: SigningKey,
    pub pk: Binary,
}

impl TestAccount {
    pub fn new_random(code_hash: &Hash256, salt: &[u8]) -> Self {
        let address = Addr::compute(&GENESIS_SENDER, code_hash, salt);
        let sk = SigningKey::random(&mut OsRng);
        let pk = sk
            .verifying_key()
            .to_encoded_point(true)
            .to_bytes()
            .to_vec()
            .into();

        Self { address, sk, pk }
    }

    pub fn sign_transaction(
        &self,
        msgs: Vec<Message>,
        gas_limit: u64,
        chain_id: &str,
        sequence: u32,
    ) -> anyhow::Result<Tx> {
        let sign_bytes = Identity256::from(grug_account::make_sign_bytes(
            sha2_256,
            &msgs,
            &self.address,
            chain_id,
            sequence,
        )?);

        let signature: Signature = self.sk.sign_digest(sign_bytes);

        let credential = to_json_value(&Credential {
            signature: signature.to_vec().into(),
            sequence,
        })?;

        Ok(Tx {
            sender: self.address.clone(),
            msgs,
            gas_limit,
            credential,
        })
    }
}
