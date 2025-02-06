use {
    crate::{create_signature, TestSuite},
    dango_types::{
        account::single,
        account_factory::{
            self, AccountParams, AccountType, NewUserSalt, QueryCodeHashRequest,
            QueryNextAccountIndexRequest, Salt, Username,
        },
        auth::{Credential, Key, Metadata, SignDoc, Signature},
    },
    grug::{
        Addr, Addressable, Coins, Defined, Duration, Hash256, Json, JsonSerExt, MaybeDefined,
        Message, NonEmpty, QuerierExt, ResultExt, Signer, StdResult, Tx, Undefined, UnsignedTx,
    },
    grug_app::{AppError, ProposalPreparer},
    k256::{ecdsa::SigningKey, elliptic_curve::rand_core::OsRng},
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

#[derive(Debug)]
pub struct TestAccount<T: MaybeDefined<Addr> = Defined<Addr>> {
    pub username: Username,
    pub nonce: u32,
    pub sk: SigningKey, // sk = signing key or secret key
    pub pk: Key,        // pk = public key
    address: T,
}

impl TestAccount<Undefined<Addr>> {
    pub fn new_random(username: &str) -> Self {
        let sk = SigningKey::random(&mut OsRng);

        Self::new(username, sk)
    }

    pub fn new_from_private_key(username: &str, sk_bytes: [u8; 32]) -> Self {
        let sk = SigningKey::from_bytes(&sk_bytes.into()).unwrap();

        Self::new(username, sk)
    }

    fn new(username: &str, sk: SigningKey) -> Self {
        let username = Username::from_str(username).unwrap();
        let pk = sk
            .verifying_key()
            .to_encoded_point(true)
            .to_bytes()
            .to_vec()
            .try_into()
            .unwrap();

        Self {
            username,
            nonce: 0,
            address: Undefined::new(),
            sk,
            pk: Key::Secp256k1(pk),
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
                key: self.pk,
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
            sk: self.sk,
            pk: self.pk,
        }
    }

    pub fn set_address(self, addresses: &BTreeMap<Username, Addr>) -> TestAccount {
        TestAccount {
            address: Defined::new(addresses[&self.username]),
            username: self.username,
            nonce: self.nonce,
            sk: self.sk,
            pk: self.pk,
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
        let standard_credential = self.create_standard_credential(&sign_doc.to_json_vec()?);

        Ok((data, Credential::Standard(standard_credential)))
    }

    pub fn create_standard_credential(&self, sign_bytes: &[u8]) -> Signature {
        let signature = create_signature(&self.sk, sign_bytes);

        Signature::Secp256k1(signature)
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
            sk: self.sk.clone(),
            pk: self.pk,
        })
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
