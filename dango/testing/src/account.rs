use {
    dango_types::{
        account_factory::{NewUserSalt, Username},
        auth::{Credential, Key, Metadata, SignDoc},
    },
    grug::{
        Addr, Addressable, Defined, Hash160, Hash256, HashExt, Json, JsonSerExt, MaybeDefined,
        Message, Signer, StdResult, Tx, Undefined,
    },
    k256::{
        ecdsa::{signature::Signer as SignerTrait, Signature, SigningKey},
        elliptic_curve::rand_core::OsRng,
    },
    std::{collections::BTreeMap, str::FromStr},
};

pub struct Accounts {
    pub owner: TestAccount,
    pub fee_recipient: TestAccount,
    pub relayer: TestAccount,
}

// ------------------------------- test account --------------------------------

#[derive(Debug)]
pub struct TestAccount<T: MaybeDefined<Inner = Addr> = Defined<Addr>> {
    pub username: Username,
    pub key: Key,
    pub key_hash: Hash160,
    pub sequence: u32,
    sk: SigningKey,
    address: T,
}

impl TestAccount<Undefined<Addr>> {
    pub fn new_random(username: &str) -> StdResult<Self> {
        // Generate a random Secp256k1 key pair.
        let sk = SigningKey::random(&mut OsRng);
        let pk = sk
            .verifying_key()
            .to_encoded_point(true)
            .to_bytes()
            .to_vec()
            .try_into()?;

        let username = Username::from_str(username)?;
        let key = Key::Secp256k1(pk);
        let key_hash = pk.hash160();

        Ok(Self {
            username,
            key,
            key_hash,
            sequence: 0,
            sk,
            address: Undefined::default(),
        })
    }

    pub fn predict_address(
        self,
        factory: Addr,
        spot_code_hash: Hash256,
        new_user_salt: bool,
    ) -> StdResult<TestAccount> {
        let salt = if new_user_salt {
            NewUserSalt {
                username: &self.username,
                key: self.key,
                key_hash: self.key_hash,
            }
            .into_bytes()
        } else {
            todo!("implement address prediction for not new users");
        };

        let address = Addr::compute(factory, spot_code_hash, &salt);

        Ok(TestAccount {
            username: self.username,
            key: self.key,
            key_hash: self.key_hash,
            sequence: self.sequence,
            sk: self.sk,
            address: Defined::new(address),
        })
    }

    pub fn set_address(self, addresses: &BTreeMap<Username, Addr>) -> TestAccount {
        let address = addresses[&self.username];

        TestAccount {
            username: self.username,
            key: self.key,
            key_hash: self.key_hash,
            sequence: self.sequence,
            sk: self.sk,
            address: Defined::new(address),
        }
    }
}

impl<T> TestAccount<T>
where
    T: MaybeDefined<Inner = Addr>,
{
    fn sign_transaction_with_sequence(
        &self,
        msgs: Vec<Message>,
        chain_id: &str,
        sequence: u32,
    ) -> StdResult<(Metadata, Credential)> {
        let sign_bytes = SignDoc {
            messages: msgs.clone(),
            chain_id: chain_id.to_string(),
            sequence,
        }
        .to_json_vec()?;

        // This hashes `sign_doc_raw` with SHA2-256. If we eventually choose to
        // use another hash, it's necessary to update this.
        let signature: Signature = self.sk.sign(&sign_bytes);

        let data = Metadata {
            username: self.username.clone(),
            key_hash: self.key_hash,
            sequence,
        };

        let credential = Credential::Secp256k1(signature.to_bytes().to_vec().try_into()?);

        Ok((data, credential))
    }
}

impl Addressable for TestAccount<Defined<Addr>> {
    fn address(&self) -> Addr {
        *self.address.inner()
    }
}

impl Signer for TestAccount<Defined<Addr>> {
    fn sign_transaction(
        &mut self,
        msgs: Vec<Message>,
        chain_id: &str,
        gas_limit: u64,
    ) -> StdResult<Tx> {
        let (data, credential) =
            self.sign_transaction_with_sequence(msgs.clone(), chain_id, self.sequence)?;

        // Increment the internally tracked sequence.
        self.sequence += 1;

        Ok(Tx {
            sender: self.address(),
            gas_limit,
            msgs,
            data: data.to_json_value()?,
            credential: credential.to_json_value()?,
        })
    }
}

// ---------------------------------- factory ----------------------------------

pub struct Factory {
    address: Addr,
}

impl Factory {
    pub fn new(address: Addr) -> Self {
        Self { address }
    }
}

impl Addressable for Factory {
    fn address(&self) -> Addr {
        self.address
    }
}

impl Signer for Factory {
    fn sign_transaction(
        &mut self,
        msgs: Vec<Message>,
        _chain_id: &str,
        gas_limit: u64,
    ) -> StdResult<Tx> {
        Ok(Tx {
            sender: self.address,
            gas_limit,
            msgs,
            data: Json::Null,
            credential: Json::Null,
        })
    }
}

// ----------------------------------- safe ------------------------------------

pub struct Safe<'a> {
    address: Addr,
    signer: Option<&'a TestAccount>,
    sequence: u32,
}

impl<'a> Safe<'a> {
    pub fn new(address: Addr) -> Self {
        Self {
            address,
            signer: None,
            sequence: 0,
        }
    }
}

impl<'a> Safe<'a> {
    pub fn with_signer(&mut self, signer: &'a TestAccount) -> &mut Self {
        self.signer = Some(signer);
        self
    }
}

impl<'a> Addressable for Safe<'a> {
    fn address(&self) -> Addr {
        self.address
    }
}

impl<'a> Signer for Safe<'a> {
    fn sign_transaction(
        &mut self,
        msgs: Vec<Message>,
        chain_id: &str,
        gas_limit: u64,
    ) -> StdResult<Tx> {
        let (data, credential) = self
            .signer
            .expect("[Safe]: signer not set")
            .sign_transaction_with_sequence(msgs.clone(), chain_id, self.sequence)?;

        // Increment the internally tracked sequence.
        self.sequence += 1;

        Ok(Tx {
            sender: self.address,
            gas_limit,
            msgs,
            data: data.to_json_value()?,
            credential: credential.to_json_value()?,
        })
    }
}
