use {
    dango_testing::setup_test_naive,
    dango_types::{
        account::single::Params,
        account_factory::{AccountParams, Username},
        constants::USDC_DENOM,
    },
    grug::{Addressable, Coin, Coins, Duration, ResultExt},
    session_account::SessionAccount,
    std::str::FromStr,
};

mod session_account {
    use {
        dango_testing::{TestAccount, create_signature, generate_random_key},
        dango_types::auth::{
            Credential, Metadata, SessionCredential, SessionInfo, SignDoc, Signature,
            StandardCredential,
        },
        grug::{
            Addr, Addressable, ByteArray, Defined, JsonSerExt, Message, NonEmpty, Signer,
            StdResult, Timestamp, Tx, Undefined, UnsignedTx,
        },
        k256::ecdsa::SigningKey,
        std::ops::{Deref, DerefMut},
    };

    /// Contains both SessionInfo and the SessionInfo signed with the user keys.
    #[derive(Clone)]
    pub struct SessionInfoBuffer {
        pub session_info: SessionInfo,
        pub sign_info_signature: Signature,
    }

    pub struct SessionAccount<T> {
        pub account: TestAccount,
        session_sk: SigningKey,
        session_pk: ByteArray<33>,
        /// Contains both SessionInfo and the SessionInfo signed with the user keys.
        session_buffer: T,
    }

    impl SessionAccount<Undefined<SessionInfoBuffer>> {
        pub fn new(account: TestAccount) -> Self {
            let (session_sk, session_pk) = generate_random_key();

            Self {
                account,
                session_sk,
                session_pk,
                session_buffer: Undefined::default(),
            }
        }

        /// Create a new account copying the session key from another account.
        /// it's used to simulate 2 accounts under the same username sharing the same session key.
        pub fn new_from_same_username(
            other: &SessionAccount<Defined<SessionInfoBuffer>>,
            account: TestAccount,
        ) -> SessionAccount<Defined<SessionInfoBuffer>> {
            SessionAccount::<Defined<SessionInfoBuffer>> {
                account,
                session_sk: other.session_sk.clone(),
                session_pk: other.session_pk,
                session_buffer: other.session_buffer.clone(),
            }
        }
    }

    impl<T> SessionAccount<T> {
        /// Generate a new session key.
        pub fn refresh_session_key(
            self,
        ) -> anyhow::Result<SessionAccount<Undefined<SessionInfoBuffer>>> {
            let (session_sk, session_pk) = generate_random_key();

            Ok(SessionAccount {
                account: self.account,
                session_sk,
                session_pk,
                session_buffer: Undefined::default(),
            })
        }

        // Sign the `SessionInfo` with the username key.
        pub fn sign_session_key(
            self,
            expire_at: Timestamp,
        ) -> anyhow::Result<SessionAccount<Defined<SessionInfoBuffer>>> {
            let session_info = SessionInfo {
                session_key: self.session_pk,
                expire_at,
            };

            // Convert to JSON value first such that the struct fields are sorted alphabetically.
            let sign_bytes = session_info.to_json_value()?.to_json_vec()?;

            let credential = self.account.create_standard_credential(&sign_bytes);

            let session_buffer = SessionInfoBuffer {
                session_info,
                sign_info_signature: credential.signature,
            };

            Ok(SessionAccount {
                account: self.account,
                session_sk: self.session_sk,
                session_pk: self.session_pk,
                session_buffer: Defined::new(session_buffer),
            })
        }
    }

    impl<T> Deref for SessionAccount<T> {
        type Target = TestAccount;

        fn deref(&self) -> &Self::Target {
            &self.account
        }
    }

    impl<T> DerefMut for SessionAccount<T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.account
        }
    }

    impl<T> Addressable for SessionAccount<T> {
        fn address(&self) -> Addr {
            self.account.address()
        }
    }

    impl Signer for SessionAccount<Defined<SessionInfoBuffer>> {
        fn unsigned_transaction(
            &self,
            _msgs: NonEmpty<Vec<Message>>,
            _chain_id: &str,
        ) -> StdResult<UnsignedTx> {
            unimplemented!("not used in this particular test");
        }

        fn sign_transaction(
            &mut self,
            msgs: NonEmpty<Vec<Message>>,
            chain_id: &str,
            gas_limit: u64,
        ) -> StdResult<Tx> {
            let data = Metadata {
                username: self.username.clone(),
                chain_id: chain_id.to_string(),
                nonce: self.nonce,
                expiry: None,
            };

            let sign_doc = SignDoc {
                gas_limit,
                sender: self.address(),
                messages: msgs.clone(),
                data: data.clone(),
            };

            // Convert to JSON value first such that the struct fields are sorted alphabetically.
            let sign_bytes = sign_doc.to_json_value()?.to_json_vec()?;

            let standard_credential = StandardCredential {
                key_hash: self.sign_with(),
                signature: self.session_buffer.inner().sign_info_signature.clone(),
            };

            let session_signature = create_signature(&self.session_sk, &sign_bytes);

            let credential = Credential::Session(SessionCredential {
                session_info: self.session_buffer.inner().session_info.clone(),
                session_signature,
                authorization: standard_credential,
            });

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
}

#[test]
fn session_key() {
    let (mut suite, accounts, _, contracts) = setup_test_naive();

    suite.block_time = Duration::from_seconds(10);

    let mut owner = SessionAccount::new(accounts.owner)
        .sign_session_key(suite.block.timestamp + Duration::from_seconds(100))
        .unwrap();

    // Ok transfer
    {
        suite
            .transfer(
                &mut owner,
                accounts.user1.address(),
                Coin::new(USDC_DENOM.clone(), 100).unwrap(),
            )
            .should_succeed();
    }

    // Expire the timestamp
    {
        suite.block_time = Duration::from_seconds(91);
        suite
            .transfer(
                &mut owner,
                accounts.user1.address(),
                Coin::new(USDC_DENOM.clone(), 100).unwrap(),
            )
            .should_fail_with_error("session expired at Duration(Dec(Int(31536100000000000))");
        owner.nonce -= 1;

        suite.block_time = Duration::from_seconds(10);
    }

    // Sign the session key again refreshing the timestamp
    {
        owner = owner
            .sign_session_key(suite.block.timestamp + Duration::from_seconds(100))
            .unwrap();

        suite
            .transfer(
                &mut owner,
                accounts.user1.address(),
                Coin::new(USDC_DENOM.clone(), 100).unwrap(),
            )
            .should_succeed();
    }

    // Try use the same session key signature with a different account.
    // We need to create a new account under the same username.
    // Then create a new SessionAccount with new_from_same_username,
    // which will use the same session key signature generated with owner1.
    {
        let owner2 = owner
            .register_new_account(
                &mut suite,
                contracts.account_factory,
                AccountParams::Spot(Params::new(Username::from_str("owner").unwrap())),
                Coins::default(),
            )
            .unwrap();

        // Refresh the session key signature
        owner = owner
            .sign_session_key(suite.block.timestamp + Duration::from_seconds(100))
            .unwrap();

        // Create a SessionAccount from the new account
        // using the same session key signature of the first account.
        let mut owner2 = SessionAccount::new_from_same_username(&owner, owner2);

        // Send some coins to the new account
        suite
            .transfer(
                &mut owner,
                owner2.address(),
                Coin::new(USDC_DENOM.clone(), 100).unwrap(),
            )
            .should_succeed();

        // The new account should be able to send coins to the relayer
        suite
            .transfer(
                &mut owner2,
                accounts.user1.address(),
                Coin::new(USDC_DENOM.clone(), 100).unwrap(),
            )
            .should_succeed();
    }

    // Generate a new fresh session_key
    {
        owner = owner
            .refresh_session_key()
            .unwrap()
            .sign_session_key(suite.block.timestamp + Duration::from_seconds(100))
            .unwrap();

        // Send some coins to the relayer
        suite
            .transfer(
                &mut owner,
                accounts.user1.address(),
                Coin::new(USDC_DENOM.clone(), 100).unwrap(),
            )
            .should_succeed();
    }
}
