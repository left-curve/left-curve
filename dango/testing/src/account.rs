use {
    crate::TestSuite,
    dango_types::{
        account::single,
        account_factory::{
            self, AccountParams, AccountType, NewUserSalt, QueryCodeHashRequest,
            QueryNextAccountIndexRequest, Salt, Username,
        },
        auth::{Credential, Key, Metadata, SignDoc, Signature, StandardCredential},
    },
    grug::{
        btree_map, Addr, Addressable, ByteArray, Coins, Defined, Hash256, HashExt, Json,
        JsonSerExt, MaybeDefined, Message, NonEmpty, ResultExt, Signer, StdResult, Tx, Undefined,
        UnsignedTx,
    },
    grug_app::{AppError, ProposalPreparer},
    k256::{
        ecdsa::{signature::Signer as SignerTrait, Signature as EcdsaSignature, SigningKey},
        elliptic_curve::rand_core::OsRng,
    },
    std::{collections::BTreeMap, str::FromStr},
};

pub struct Accounts {
    pub owner: TestAccount,
    pub relayer: TestAccount,
}

// ------------------------------- test account --------------------------------

#[derive(Debug)]
pub struct TestAccount<
    T: MaybeDefined<Addr> = Defined<Addr>,
    K = BTreeMap<Hash256, (SigningKey, Key)>,
> {
    pub username: Username,
    pub nonce: u32,
    keys: K,
    sign_with: Hash256,
    address: T,
}

impl TestAccount<Undefined<Addr>, (SigningKey, Key)> {
    pub fn new_random(username: &str) -> Self {
        let (sk, pk) = generate_random_key();
        let username = Username::from_str(username).unwrap();
        let key = Key::Secp256k1(pk);
        let key_hash = pk.hash256();

        Self {
            username,
            nonce: 0,
            address: Undefined::new(),
            keys: (sk, key),
            sign_with: key_hash,
        }
    }

    pub fn predict_address(
        self,
        factory: Addr,
        spot_code_hash: Hash256,
        new_user_salt: bool,
    ) -> TestAccount {
        let salt = if new_user_salt {
            NewUserSalt {
                username: &self.username,
                key: self.keys.1,
                key_hash: self.sign_with,
            }
            .into_bytes()
        } else {
            todo!("implement address prediction for not new users");
        };

        let address = Addr::derive(factory, spot_code_hash, &salt);

        TestAccount {
            username: self.username,
            nonce: self.nonce,
            address: Defined::new(address),
            keys: btree_map!(self.sign_with => self.keys),
            sign_with: self.sign_with,
        }
    }

    pub fn set_address(self, addresses: &BTreeMap<Username, Addr>) -> TestAccount {
        let address = addresses[&self.username];

        TestAccount {
            username: self.username,
            nonce: self.nonce,
            address: Defined::new(address),
            keys: btree_map!(self.sign_with => self.keys),
            sign_with: self.sign_with,
        }
    }
}

impl<T> TestAccount<T>
where
    T: MaybeDefined<Addr>,
{
    pub fn metadata(&self, chain_id: &str, nonce: u32) -> Metadata {
        Metadata {
            username: self.username.clone(),
            chain_id: chain_id.to_string(),
            expiry: None,
            nonce,
        }
    }

    pub fn sign_transaction_with_nonce(
        &self,
        sender: Addr,
        msgs: NonEmpty<Vec<Message>>,
        chain_id: &str,
        gas_limit: u64,
        nonce: u32,
    ) -> StdResult<(Metadata, Credential)> {
        let data = self.metadata(chain_id, nonce);
        let sign_bytes = SignDoc {
            sender,
            gas_limit,
            messages: msgs.clone(),
            data: data.clone(),
        }
        .to_json_vec()?;

        let standard_credential = self.create_standard_credential(&sign_bytes)?;

        Ok((data, Credential::Standard(standard_credential)))
    }

    pub fn create_standard_credential(&self, sign_bytes: &[u8]) -> StdResult<StandardCredential> {
        let sk = &self.keys.get(&self.sign_with).unwrap().0;

        let signature = create_signature(sk, sign_bytes)?;

        Ok(StandardCredential {
            key_hash: self.sign_with,
            signature: Signature::Secp256k1(signature),
        })
    }
}

impl<T> TestAccount<T>
where
    T: MaybeDefined<Addr>,
    Self: Signer,
{
    /// Register a new account with the username and key of this account and returns a new
    /// `TestAccount` with the new account's address.
    pub fn register_new_account<PP>(
        &mut self,
        test_suite: &mut TestSuite<PP>,
        factory: Addr,
        params: AccountParams,
        funds: Coins,
    ) -> StdResult<TestAccount>
    where
        PP: ProposalPreparer,
        AppError: From<PP::Error>,
    {
        // If registering a single account, ensure the supplied username matches this account's username.
        let account_type = match &params {
            AccountParams::Spot(single::Params { owner, .. }) => {
                assert_eq!(owner, &self.username);
                AccountType::Spot
            },
            AccountParams::Margin(single::Params { owner, .. }) => {
                assert_eq!(owner, &self.username);
                AccountType::Margin
            },
            AccountParams::Safe(_) => AccountType::Safe,
        };

        // Derive the new accounts address.
        let index = test_suite
            .query_wasm_smart(factory, QueryNextAccountIndexRequest {})
            .unwrap();

        let code_hash = test_suite
            .query_wasm_smart(factory, QueryCodeHashRequest { account_type })
            .should_succeed();

        let address = Addr::derive(factory, code_hash, Salt { index }.into_bytes().as_slice());

        // Create a new account
        test_suite
            .execute(
                &mut *self,
                factory,
                &account_factory::ExecuteMsg::RegisterAccount { params },
                funds,
            )
            .should_succeed();

        Ok(TestAccount {
            username: self.username.clone(),
            nonce: 0,
            address: Defined::new(address),
            keys: self.keys.clone(),
            sign_with: self.sign_with,
        })
    }
}

impl<T> TestAccount<T, (SigningKey, Key)>
where
    T: MaybeDefined<Addr>,
{
    pub fn key(&self) -> &Key {
        &self.keys.1
    }

    pub fn key_hash(&self) -> Hash256 {
        self.sign_with
    }
}

impl<T> TestAccount<T>
where
    T: MaybeDefined<Addr>,
{
    pub fn first_key(&self) -> Key {
        self.keys.iter().next().unwrap().1 .1
    }

    pub fn first_key_hash(&self) -> Hash256 {
        *self.keys.keys().next().unwrap()
    }

    pub fn keys(&self) -> &BTreeMap<Hash256, (SigningKey, Key)> {
        &self.keys
    }

    pub fn sign_with(&self) -> Hash256 {
        self.sign_with
    }
}

impl Addressable for TestAccount {
    fn address(&self) -> Addr {
        *self.address.inner()
    }
}

impl Signer for TestAccount {
    fn unsigned_transaction(
        &self,
        msgs: NonEmpty<Vec<Message>>,
        chain_id: &str,
    ) -> StdResult<UnsignedTx> {
        Ok(UnsignedTx {
            sender: self.address(),
            msgs,
            data: self.metadata(chain_id, self.nonce).to_json_value()?,
        })
    }

    fn sign_transaction(
        &mut self,
        msgs: NonEmpty<Vec<Message>>,
        chain_id: &str,
        gas_limit: u64,
    ) -> StdResult<Tx> {
        let (data, credential) = self.sign_transaction_with_nonce(
            self.address(),
            msgs.clone(),
            chain_id,
            gas_limit,
            self.nonce,
        )?;

        // Increment the internally tracked nonce.
        self.nonce += 1;

        Ok(Tx {
            sender: self.address(),
            gas_limit,
            msgs,
            data: data.to_json_value()?,
            credential: credential.to_json_value()?,
        })
    }
}

pub fn generate_random_key() -> (SigningKey, ByteArray<33>) {
    let sk = SigningKey::random(&mut OsRng);
    let pk = sk
        .verifying_key()
        .to_encoded_point(true)
        .to_bytes()
        .to_vec()
        .try_into()
        .unwrap();
    (sk, pk)
}

pub fn create_signature(sk: &SigningKey, sign_bytes: &[u8]) -> StdResult<ByteArray<64>> {
    // This hashes `sign_doc_raw` with SHA2-256. If we eventually choose to
    // use another hash, it's necessary to update this.
    let signature: EcdsaSignature = sk.sign(sign_bytes);
    signature.to_bytes().to_vec().try_into()
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
    fn unsigned_transaction(
        &self,
        msgs: NonEmpty<Vec<Message>>,
        _chain_id: &str,
    ) -> StdResult<UnsignedTx> {
        Ok(UnsignedTx {
            sender: self.address(),
            msgs,
            data: Json::null(),
        })
    }

    fn sign_transaction(
        &mut self,
        msgs: NonEmpty<Vec<Message>>,
        _chain_id: &str,
        gas_limit: u64,
    ) -> StdResult<Tx> {
        Ok(Tx {
            sender: self.address,
            gas_limit,
            msgs,
            data: Json::null(),
            credential: Json::null(),
        })
    }
}

// ----------------------------------- safe ------------------------------------

pub struct Safe<'a> {
    address: Addr,
    signer: Option<&'a TestAccount>,
    nonce: u32,
}

impl Safe<'_> {
    pub fn new(address: Addr) -> Self {
        Self {
            address,
            signer: None,
            nonce: 0,
        }
    }
}

impl<'a> Safe<'a> {
    pub fn with_signer(&mut self, signer: &'a TestAccount) -> &mut Self {
        self.signer = Some(signer);
        self
    }

    pub fn with_nonce(&mut self, nonce: u32) -> &mut Self {
        self.nonce = nonce;
        self
    }
}

impl Addressable for Safe<'_> {
    fn address(&self) -> Addr {
        self.address
    }
}

impl Signer for Safe<'_> {
    fn unsigned_transaction(
        &self,
        msgs: NonEmpty<Vec<Message>>,
        chain_id: &str,
    ) -> StdResult<UnsignedTx> {
        self.signer
            .expect("[Safe]: signer not set")
            .unsigned_transaction(msgs, chain_id)
    }

    fn sign_transaction(
        &mut self,
        msgs: NonEmpty<Vec<Message>>,
        chain_id: &str,
        gas_limit: u64,
    ) -> StdResult<Tx> {
        let (data, credential) = self
            .signer
            .expect("[Safe]: signer not set")
            .sign_transaction_with_nonce(
                self.address(),
                msgs.clone(),
                chain_id,
                gas_limit,
                self.nonce,
            )?;

        // Increment the internally tracked nonce.
        self.nonce += 1;

        Ok(Tx {
            sender: self.address,
            gas_limit,
            msgs,
            data: data.to_json_value()?,
            credential: credential.to_json_value()?,
        })
    }
}
