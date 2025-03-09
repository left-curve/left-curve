use {
    grug_crypto::{Identity256, sha2_256},
    grug_mock_account::{Credential, PublicKey},
    grug_types::{
        Addr, Addressable, ByteArray, GENESIS_SENDER, Hash256, Json, JsonSerExt, Message, NonEmpty,
        Signer, StdResult, Tx, UnsignedTx,
    },
    k256::ecdsa::{Signature, SigningKey, signature::DigestSigner},
    rand::rngs::OsRng,
    std::{
        collections::HashMap,
        ops::{Deref, DerefMut, Index, IndexMut},
    },
};

// ---------------------------------- account ----------------------------------

/// A signer that tracks a sequence number and signs transactions in a way
/// corresponding to the mock account used in Grug test suite.
#[derive(Debug)]
pub struct TestAccount {
    pub address: Addr,
    pub sk: SigningKey,
    pub pk: PublicKey,
    pub sequence: u32,
}

impl TestAccount {
    /// Create a new test account with a random Secp256k1 key pair.
    ///
    /// The address is predicted with the given code hash and salt, assuming the
    /// account is to be instantiated during genesis.
    pub fn new_random(code_hash: Hash256, salt: &[u8]) -> Self {
        let address = Addr::derive(GENESIS_SENDER, code_hash, salt);
        let sk = SigningKey::random(&mut OsRng);
        let pk = sk
            .verifying_key()
            .to_encoded_point(true)
            .to_bytes()
            .as_ref()
            .try_into()
            .expect("pk is of wrong length");

        Self {
            address,
            sk,
            pk: PublicKey::from_inner(pk),
            sequence: 0,
        }
    }

    /// Sign a transaction with the given sequence, without considering or
    /// updating the internally tracked sequence.
    pub fn sign_transaction_with_sequence(
        &self,
        msgs: NonEmpty<Vec<Message>>,
        chain_id: &str,
        sequence: u32,
        gas_limit: u64,
    ) -> StdResult<Tx> {
        let sign_bytes = Identity256::from(grug_mock_account::make_sign_bytes(
            sha2_256,
            &msgs,
            self.address,
            chain_id,
            sequence,
        )?);

        let signature: Signature = self.sk.sign_digest(sign_bytes);

        let credential = Credential {
            signature: ByteArray::from_inner(signature.to_vec().as_slice().try_into()?),
            sequence,
        }
        .to_json_value()?;

        Ok(Tx {
            sender: self.address,
            gas_limit,
            msgs,
            data: Json::null(),
            credential,
        })
    }
}

impl Addressable for TestAccount {
    fn address(&self) -> Addr {
        self.address
    }
}

impl Signer for TestAccount {
    fn unsigned_transaction(
        &self,
        msgs: NonEmpty<Vec<Message>>,
        _chain_id: &str,
    ) -> StdResult<UnsignedTx> {
        Ok(UnsignedTx {
            sender: self.address,
            msgs,
            data: Json::null(),
        })
    }

    fn sign_transaction(
        &mut self,
        msgs: NonEmpty<Vec<Message>>,
        chain_id: &str,
        gas_limit: u64,
    ) -> StdResult<Tx> {
        let sequence = self.sequence;
        self.sequence += 1;
        self.sign_transaction_with_sequence(msgs, chain_id, sequence, gas_limit)
    }
}

// --------------------------------- accounts ----------------------------------

/// A set of test accounts, indexed by names.
///
/// ## Note
///
/// Why not just use a `HashMap`?
///
/// The Rust `HashMap` doesn't implement `IndexMut`, so we can't index into it
/// like `&mut accounts["name"]`. We have to do `accounts.get_mut("name").unwrap()`
/// instead which is quite verbose.
///
/// To fix this, we make a wrapper over `HashMap` and implement `IndexMut` ourselves.
#[derive(Default, Debug)]
pub struct TestAccounts(HashMap<&'static str, TestAccount>);

impl Deref for TestAccounts {
    type Target = HashMap<&'static str, TestAccount>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TestAccounts {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<S> Index<S> for TestAccounts
where
    S: AsRef<str>,
{
    type Output = TestAccount;

    fn index(&self, index: S) -> &Self::Output {
        self.get(index.as_ref()).expect("account not found")
    }
}

impl<S> IndexMut<S> for TestAccounts
where
    S: AsRef<str>,
{
    fn index_mut(&mut self, index: S) -> &mut Self::Output {
        self.get_mut(index.as_ref()).expect("account not found")
    }
}
