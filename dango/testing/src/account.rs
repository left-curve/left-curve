use {
    crate::{TestSuite, create_signature},
    dango_types::{
        account::single,
        account_factory::{
            self, AccountParams, AccountType, NewUserSalt, QueryCodeHashRequest,
            QueryNextAccountIndexRequest, Salt, Username,
        },
        auth::{Credential, Key, Metadata, SignDoc, Signature, StandardCredential},
    },
    digest::{consts::U32, generic_array::GenericArray},
    grug::{
        Addr, Addressable, Coins, Defined, Duration, Hash256, HashExt, Json, JsonSerExt,
        MaybeDefined, Message, NonEmpty, QuerierExt, ResultExt, SignData, Signer, StdError,
        StdResult, Tx, Undefined, UnsignedTx, btree_map,
    },
    grug_app::{AppError, ProposalPreparer},
    k256::{ecdsa::SigningKey, elliptic_curve::rand_core::OsRng},
    sha2::Sha256,
    std::{array, collections::BTreeMap, str::FromStr},
};

/// Accounts available for testing purposes.
pub struct TestAccounts {
    pub owner: TestAccount,
    pub user1: TestAccount,
    pub user2: TestAccount,
    pub user3: TestAccount,
    pub user4: TestAccount,
    pub user5: TestAccount,
    pub user6: TestAccount,
    pub user7: TestAccount,
    pub user8: TestAccount,
    pub user9: TestAccount,
}

impl TestAccounts {
    /// Iterate over all user accounts (the owner excluded).
    pub fn users(&self) -> array::IntoIter<&TestAccount, 9> {
        [
            &self.user1,
            &self.user2,
            &self.user3,
            &self.user4,
            &self.user5,
            &self.user6,
            &self.user7,
            &self.user8,
            &self.user9,
        ]
        .into_iter()
    }

    /// Iterate over all user accounts as mutable (the owner excluded).
    pub fn users_mut(&mut self) -> array::IntoIter<&mut TestAccount, 9> {
        [
            &mut self.user1,
            &mut self.user2,
            &mut self.user3,
            &mut self.user4,
            &mut self.user5,
            &mut self.user6,
            &mut self.user7,
            &mut self.user8,
            &mut self.user9,
        ]
        .into_iter()
    }
}

// ------------------------------- test account --------------------------------

#[derive(Debug, Clone)]
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
    pub fn new_key_pair() -> (SigningKey, Key) {
        let sk = SigningKey::random(&mut OsRng);
        let pk = sk
            .verifying_key()
            .to_encoded_point(true)
            .to_bytes()
            .to_vec()
            .try_into()
            .unwrap();

        (sk, Key::Secp256k1(pk))
    }

    pub fn new_random(username: &str) -> Self {
        let sk = SigningKey::random(&mut OsRng);

        Self::new(username, sk)
    }

    pub fn new_from_private_key(username: &str, sk_bytes: [u8; 32]) -> Self {
        let sk = SigningKey::from_bytes(&sk_bytes.into()).unwrap();

        Self::new(username, sk)
    }

    pub fn new(username: &str, sk: SigningKey) -> Self {
        let username = Username::from_str(username).unwrap();
        let pk = sk
            .verifying_key()
            .to_encoded_point(true)
            .to_bytes()
            .to_vec()
            .try_into()
            .unwrap();
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
        seed: u32,
        spot_code_hash: Hash256,
        new_user_salt: bool,
    ) -> TestAccount {
        let salt = if new_user_salt {
            NewUserSalt {
                key: self.keys.1,
                key_hash: self.sign_with,
                seed,
            }
            .to_bytes()
        } else {
            todo!("implement address prediction for not new users");
        };

        let address = Addr::derive(factory, spot_code_hash, &salt);

        TestAccount {
            username: self.username,
            nonce: self.nonce,
            address: Defined::new(address),
            keys: btree_map! { self.sign_with => self.keys },
            sign_with: self.sign_with,
        }
    }

    pub fn set_address(self, addresses: &BTreeMap<Username, Addr>) -> TestAccount {
        TestAccount {
            address: Defined::new(addresses[&self.username]),
            username: self.username,
            nonce: self.nonce,
            keys: btree_map! { self.sign_with => self.keys },
            sign_with: self.sign_with,
        }
    }
}

impl<T> TestAccount<T>
where
    T: MaybeDefined<Addr>,
{
    pub fn metadata(&self, chain_id: &str, nonce: u32, expiry: Option<Duration>) -> Metadata {
        Metadata {
            username: self.username.clone(),
            chain_id: chain_id.to_string(),
            expiry,
            nonce,
        }
    }

    // TODO: currently only support sign data that use SHA256 hasher.
    pub fn sign_arbitrary<D>(&self, data: D) -> StdResult<Signature>
    where
        D: SignData<Hasher = Sha256>,
        StdError: From<D::Error>,
    {
        let bytes = data.to_sign_data()?;
        let standard_credential = self.create_standard_credential(bytes);

        Ok(standard_credential.signature)
    }

    pub fn sign_transaction_with_nonce(
        &self,
        sender: Addr,
        msgs: NonEmpty<Vec<Message>>,
        chain_id: &str,
        gas_limit: u64,
        nonce: u32,
        expiry: Option<Duration>,
    ) -> StdResult<(Metadata, Credential)> {
        let data = self.metadata(chain_id, nonce, expiry);

        let sign_doc = SignDoc {
            sender,
            gas_limit,
            messages: msgs.clone(),
            data: data.clone(),
        };

        let sign_data = sign_doc.to_sign_data()?;
        let standard_credential = self.create_standard_credential(sign_data);

        Ok((data, Credential::Standard(standard_credential)))
    }

    /// Note: This function expects the _hashed_ sign data.
    pub fn create_standard_credential(
        &self,
        sign_data: GenericArray<u8, U32>,
    ) -> StandardCredential {
        let sk = &self.keys.get(&self.sign_with).unwrap().0;
        let signature = create_signature(sk, sign_data);

        StandardCredential {
            key_hash: self.sign_with,
            signature: Signature::Secp256k1(signature),
        }
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
            AccountParams::Multi(_) => AccountType::Multi,
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
    pub fn sk(&self) -> &SigningKey {
        &self.keys.0
    }

    pub fn key(&self) -> Key {
        self.keys.1
    }

    pub fn key_hash(&self) -> Hash256 {
        self.sign_with
    }
}

impl<T> TestAccount<T>
where
    T: MaybeDefined<Addr>,
{
    pub fn first_sk(&self) -> &SigningKey {
        &self.keys.iter().next().unwrap().1.0
    }

    pub fn first_key(&self) -> Key {
        self.keys.iter().next().unwrap().1.1
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
            data: self.metadata(chain_id, self.nonce, None).to_json_value()?,
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
            None,
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

// ----------------------------------- multi -----------------------------------

pub struct Multi<'a> {
    address: Addr,
    signer: Option<&'a TestAccount>,
    nonce: u32,
}

impl Multi<'_> {
    pub fn new(address: Addr) -> Self {
        Self {
            address,
            signer: None,
            nonce: 0,
        }
    }
}

impl<'a> Multi<'a> {
    pub fn with_signer(&mut self, signer: &'a TestAccount) -> &mut Self {
        self.signer = Some(signer);
        self
    }

    pub fn with_nonce(&mut self, nonce: u32) -> &mut Self {
        self.nonce = nonce;
        self
    }
}

impl Addressable for Multi<'_> {
    fn address(&self) -> Addr {
        self.address
    }
}

impl Signer for Multi<'_> {
    fn unsigned_transaction(
        &self,
        msgs: NonEmpty<Vec<Message>>,
        chain_id: &str,
    ) -> StdResult<UnsignedTx> {
        self.signer
            .expect("[Multi]: signer not set") // TODO: use typed state pattern to avoid runtime error
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
            .expect("[Multi]: signer not set")
            .sign_transaction_with_nonce(
                self.address(),
                msgs.clone(),
                chain_id,
                gas_limit,
                self.nonce,
                None,
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
