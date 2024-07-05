use {
    grug_crypto::{sha2_256, Identity256},
    grug_types::{Addr, Binary, Hash, Message, Tx, GENESIS_SENDER},
    k256::ecdsa::{signature::DigestSigner, Signature, SigningKey},
    rand::rngs::OsRng,
    std::collections::HashMap,
};

pub type TestAccounts = HashMap<&'static str, TestAccount>;

pub struct TestAccount {
    pub address: Addr,
    pub sk: SigningKey,
    pub pk: Binary,
    pub sequence: u32,
}

impl TestAccount {
    pub fn new_random(code_hash: &Hash, salt: &[u8]) -> Self {
        let address = Addr::compute(&GENESIS_SENDER, code_hash, salt);
        let sk = SigningKey::random(&mut OsRng);
        let pk = sk
            .verifying_key()
            .to_encoded_point(true)
            .to_bytes()
            .to_vec()
            .into();

        Self {
            address,
            sk,
            pk,
            sequence: 0,
        }
    }

    pub fn sign_transaction(
        &mut self,
        msgs: Vec<Message>,
        gas_limit: u64,
        chain_id: &str,
    ) -> anyhow::Result<Tx> {
        // Sign the transaction
        let sign_bytes = Identity256::from(grug_account::make_sign_bytes(
            sha2_256,
            &msgs,
            &self.address,
            chain_id,
            self.sequence,
        )?);
        let signature: Signature = self.sk.sign_digest(sign_bytes);

        // Increment the internally-tracked sequence, for use when signing the
        // next transaction.
        self.sequence += 1;

        Ok(Tx {
            sender: self.address.clone(),
            msgs,
            gas_limit,
            credential: signature.to_vec().into(),
        })
    }
}
