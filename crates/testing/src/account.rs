use {
    grug_account::{Credential, PublicKey},
    grug_crypto::{sha2_256, Identity256},
    grug_types::{Addr, Hash256, Json, JsonSerExt, Message, StdResult, Tx, GENESIS_SENDER},
    k256::ecdsa::{signature::DigestSigner, Signature, SigningKey},
    rand::rngs::OsRng,
    std::collections::HashMap,
};

/// Describes an account that is capable of signing transactions.
pub trait Signer {
    /// Return the signer's address.
    fn address(&self) -> Addr;

    /// Given a list of messages and relevant metadata, produce a signed transaction.
    fn sign_transaction(
        &self,
        msgs: Vec<Message>,
        gas_limit: u64,
        chain_id: &str,
        sequence: u32,
    ) -> StdResult<Tx>;
}

pub struct TestAccount {
    pub address: Addr,
    pub sk: SigningKey,
    pub pk: PublicKey,
}

impl TestAccount {
    pub fn new_random(code_hash: Hash256, salt: &[u8]) -> Self {
        let address = Addr::compute(GENESIS_SENDER, code_hash, salt);
        let sk = SigningKey::random(&mut OsRng);
        let pk = sk
            .verifying_key()
            .to_encoded_point(true)
            .to_bytes()
            .to_vec()
            .try_into()
            .expect("pk is of wrong length");

        Self { address, sk, pk }
    }
}

impl Signer for TestAccount {
    fn address(&self) -> Addr {
        self.address
    }

    fn sign_transaction(
        &self,
        msgs: Vec<Message>,
        gas_limit: u64,
        chain_id: &str,
        sequence: u32,
    ) -> StdResult<Tx> {
        let sign_bytes = Identity256::from(grug_account::make_sign_bytes(
            sha2_256,
            &msgs,
            self.address,
            chain_id,
            sequence,
        )?);

        let signature: Signature = self.sk.sign_digest(sign_bytes);

        let credential = Credential {
            signature: signature.to_vec().try_into()?,
            sequence,
        }
        .to_json_value()?;

        Ok(Tx {
            sender: self.address,
            gas_limit,
            msgs,
            data: Json::Null,
            credential,
        })
    }
}

pub type TestAccounts = HashMap<&'static str, TestAccount>;
