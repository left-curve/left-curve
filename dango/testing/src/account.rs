use {
    crate::TestSuite,
    dango_types::{
        account::single,
        account_factory::{
            self, AccountParams, NewUserSalt, QueryNextAccountIndexRequest, Salt, Username,
        },
        auth::{Credential, Key, Metadata, SignDoc, Signature, StandardCredential},
    },
    grug::{
        Addr, Addressable, Coins, Defined, Hash160, Hash256, HashExt, Json, JsonSerExt,
        MaybeDefined, Message, NonEmpty, ResultExt, Signer, StdResult, Tx, Undefined,
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
pub struct TestAccount<T: MaybeDefined<Addr> = Defined<Addr>> {
    pub username: Username,
    pub key: Key,
    pub key_hash: Hash160,
    pub sequence: u32,
    pub sk: SigningKey,
    address: T,
}

impl TestAccount<Undefined<Addr>> {
    pub fn new_random(username: &str) -> Self {
        // Generate a random Secp256k1 key pair.
        let sk = SigningKey::random(&mut OsRng);
        let pk = sk
            .verifying_key()
            .to_encoded_point(true)
            .to_bytes()
            .to_vec()
            .try_into()
            .unwrap();

        let username = Username::from_str(username).unwrap();
        let key = Key::Secp256k1(pk);
        let key_hash = pk.hash160();

        Self {
            username,
            key,
            key_hash,
            sequence: 0,
            sk,
            address: Undefined::new(),
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
                key: self.key,
                key_hash: self.key_hash,
            }
            .into_bytes()
        } else {
            todo!("implement address prediction for not new users");
        };

        let address = Addr::derive(factory, spot_code_hash, &salt);

        TestAccount {
            username: self.username,
            key: self.key,
            key_hash: self.key_hash,
            sequence: self.sequence,
            sk: self.sk,
            address: Defined::new(address),
        }
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
    T: MaybeDefined<Addr>,
{
    pub fn sign_transaction_with_sequence(
        &self,
        sender: Addr,
        msgs: Vec<Message>,
        chain_id: &str,
        sequence: u32,
    ) -> StdResult<(Metadata, Credential)> {
        let sign_bytes = SignDoc {
            sender,
            messages: msgs.clone(),
            chain_id: chain_id.to_string(),
            sequence,
        }
        .to_json_vec()?;

        // This hashes `sign_doc_raw` with SHA2-256. If we eventually choose to
        // use another hash, it's necessary to update this.
        let signature: EcdsaSignature = self.sk.sign(&sign_bytes);

        let data = Metadata {
            username: self.username.clone(),
            key_hash: self.key_hash,
            sequence,
        };

        let credential = Credential::Standard(StandardCredential {
            signature: Signature::Secp256k1(signature.to_bytes().to_vec().try_into()?),
            otp: None,
        });

        Ok((data, credential))
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
        code_hash: Hash256,
        params: AccountParams,
        funds: Coins,
    ) -> StdResult<TestAccount<Defined<Addr>>>
    where
        PP: ProposalPreparer,
        AppError: From<PP::Error>,
    {
        // If registering a single account, ensure the supplied username matches this account's username.
        match &params {
            AccountParams::Spot(single::Params { owner, .. })
            | AccountParams::Margin(single::Params { owner, .. }) => {
                assert_eq!(owner, &self.username);
            },
            _ => {},
        }

        // Derive the new accounts address.
        let index = test_suite
            .query_wasm_smart(factory, QueryNextAccountIndexRequest {})
            .unwrap();
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
            key: self.key,
            key_hash: self.key_hash,
            sequence: 0,
            sk: self.sk.clone(),
            address: Defined::new(address),
        })
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
        let (data, credential) = self.sign_transaction_with_sequence(
            self.address(),
            msgs.clone(),
            chain_id,
            self.sequence,
        )?;

        // Increment the internally tracked sequence.
        self.sequence += 1;

        Ok(Tx {
            sender: self.address(),
            gas_limit,
            msgs: NonEmpty::new(msgs)?,
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
            msgs: NonEmpty::new(msgs)?,
            data: Json::null(),
            credential: Json::null(),
        })
    }
}

// ----------------------------------- safe ------------------------------------

pub struct Safe<'a> {
    address: Addr,
    signer: Option<&'a TestAccount>,
    sequence: u32,
}

impl Safe<'_> {
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

    pub fn with_sequence(&mut self, sequence: u32) -> &mut Self {
        self.sequence = sequence;
        self
    }
}

impl Addressable for Safe<'_> {
    fn address(&self) -> Addr {
        self.address
    }
}

impl Signer for Safe<'_> {
    fn sign_transaction(
        &mut self,
        msgs: Vec<Message>,
        chain_id: &str,
        gas_limit: u64,
    ) -> StdResult<Tx> {
        let (data, credential) = self
            .signer
            .expect("[Safe]: signer not set")
            .sign_transaction_with_sequence(
                self.address(),
                msgs.clone(),
                chain_id,
                self.sequence,
            )?;

        // Increment the internally tracked sequence.
        self.sequence += 1;

        Ok(Tx {
            sender: self.address,
            gas_limit,
            msgs: NonEmpty::new(msgs)?,
            data: data.to_json_value()?,
            credential: credential.to_json_value()?,
        })
    }
}
