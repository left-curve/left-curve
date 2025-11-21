use {
    crate::Secret,
    dango_types::{
        account::spot,
        account_factory::Username,
        auth::{Credential, Metadata, Nonce, SignDoc, StandardCredential},
        signer::SequencedSigner,
    },
    grug::{
        Addr, Addressable, Defined, JsonSerExt, MaybeDefined, Message, NonEmpty, QueryClient,
        QueryClientExt, Signer, StdError, StdResult, Tx, Undefined, UnsignedTx,
    },
    std::str::FromStr,
};

pub const DEFAULT_DERIVATION_PATH: &str = "m/44'/60'/0'/0/0";

/// Utility for signing transactions in the format by Dango's single-signature
/// accounts, i.e. spot and margin accounts.
#[derive(Debug)]
pub struct SingleSigner<S, N>
where
    S: Secret,
    N: MaybeDefined<Nonce>,
{
    pub username: Username,
    pub address: Addr,
    pub nonce: N,
    pub secret: S,
}

impl<S, N> SingleSigner<S, N>
where
    S: Secret,
    N: MaybeDefined<Nonce>,
{
    pub async fn query_next_nonce<C>(&self, client: &C) -> anyhow::Result<Nonce>
    where
        C: QueryClient,
        anyhow::Error: From<C::Error>,
    {
        // If the account hasn't sent any transaction yet, use 0 as nonce.
        // Otherwise, use the latest seen nonce + 1.
        let nonce = client
            .query_wasm_smart(self.address, spot::QuerySeenNoncesRequest {}, None)
            .await?
            .last()
            .map(|newest_nonce| newest_nonce + 1)
            .unwrap_or(0);

        Ok(nonce)
    }
}

impl<S> SingleSigner<S, Undefined<Nonce>>
where
    S: Secret,
{
    /// Create a new `SingleSigner` with the given secret key.
    pub fn new(username: &str, address: Addr, secret: S) -> anyhow::Result<Self> {
        let username = Username::from_str(username)?;

        Ok(Self {
            username,
            address,
            nonce: Undefined::new(),
            secret,
        })
    }
}

impl<S> SingleSigner<S, Undefined<Nonce>>
where
    S: Secret,
{
    pub fn with_nonce(self, nonce: Nonce) -> SingleSigner<S, Defined<Nonce>> {
        SingleSigner {
            username: self.username,
            address: self.address,
            nonce: Defined::new(nonce),
            secret: self.secret,
        }
    }

    /// Fetch the next nonce and return a `SingleSigner` with the nonce set.
    pub async fn with_query_nonce<C>(
        self,
        client: &C,
    ) -> anyhow::Result<SingleSigner<S, Defined<Nonce>>>
    where
        C: QueryClient,
        anyhow::Error: From<C::Error>,
    {
        let nonce = self.query_next_nonce(client).await?;

        Ok(SingleSigner {
            username: self.username,
            address: self.address,
            nonce: Defined::new(nonce),
            secret: self.secret,
        })
    }
}

impl<S, N> Addressable for SingleSigner<S, N>
where
    S: Secret,
    N: MaybeDefined<Nonce>,
{
    fn address(&self) -> Addr {
        self.address
    }
}

impl<S> Signer for SingleSigner<S, Defined<Nonce>>
where
    S: Secret,
{
    fn unsigned_transaction(
        &self,
        msgs: NonEmpty<Vec<Message>>,
        chain_id: &str,
    ) -> StdResult<UnsignedTx> {
        Ok(UnsignedTx {
            sender: self.address,
            msgs,
            data: Metadata {
                username: self.username.clone(),
                chain_id: chain_id.to_string(),
                nonce: self.nonce.into_inner(),
                expiry: None, // TODO
            }
            .to_json_value()?,
        })
    }

    fn sign_transaction(
        &mut self,
        msgs: NonEmpty<Vec<Message>>, // TODO: the method should take a `LengthBounded`
        chain_id: &str,
        gas_limit: u64,
    ) -> StdResult<Tx> {
        let nonce = self.nonce.into_inner();
        *self.nonce.inner_mut() += 1;

        let metadata = Metadata {
            username: self.username.clone(),
            chain_id: chain_id.to_string(),
            nonce,
            expiry: None, // TODO
        };

        let sign_doc = SignDoc {
            gas_limit,
            sender: self.address,
            messages: msgs.clone(),
            data: metadata.clone(),
        };

        let credential = Credential::Standard(StandardCredential {
            key_hash: self.secret.key_hash(),
            signature: self
                .secret
                .sign_transaction(sign_doc)
                .map_err(|err| StdError::host(err.to_string()))?, // TODO: better handle this error
        });

        Ok(Tx {
            sender: self.address,
            gas_limit,
            msgs,
            data: metadata.to_json_value()?,
            credential: credential.to_json_value()?,
        })
    }
}

#[async_trait::async_trait]
impl<S> SequencedSigner for SingleSigner<S, Defined<Nonce>>
where
    S: Secret + Send + Sync,
{
    async fn query_nonce<C>(&self, client: &C) -> anyhow::Result<Nonce>
    where
        C: QueryClient,
        anyhow::Error: From<C::Error>,
    {
        self.query_next_nonce(client).await
    }

    async fn update_nonce<C>(&mut self, client: &C) -> anyhow::Result<()>
    where
        C: QueryClient,
        anyhow::Error: From<C::Error>,
    {
        let nonce = self.query_next_nonce(client).await?;

        self.nonce = Defined::new(nonce);

        Ok(())
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{Eip712, Secp256k1},
        dango_account_factory::{ACCOUNTS_BY_USER, KEYS},
        dango_auth::authenticate_tx,
        dango_types::config::{AppAddresses, AppConfig},
        grug::{AuthMode, Coins, MockContext, MockQuerier, ResultExt},
    };

    #[test]
    fn sign_secp256k1_transaction_works() {
        let username = Username::from_str("alice").unwrap();
        let address = Addr::mock(0);
        let nonce = 0;
        let account_factory = Addr::mock(1);

        let mut signer = SingleSigner::new(username.as_ref(), address, Secp256k1::new_random())
            .unwrap()
            .with_nonce(nonce);

        let tx = signer
            .sign_transaction(
                NonEmpty::new_unchecked(vec![
                    Message::transfer(Addr::mock(2), Coins::one("uatom", 100).unwrap()).unwrap(),
                    Message::transfer(Addr::mock(3), Coins::one("uosmo", 500).unwrap()).unwrap(),
                ]),
                "dango-1",
                100_000_000,
            )
            .unwrap();

        let mock_querier = MockQuerier::new()
            .with_raw_contract_storage(account_factory, |storage| {
                ACCOUNTS_BY_USER
                    .insert(storage, (&username, address))
                    .unwrap();
                KEYS.save(
                    storage,
                    (&username, signer.secret.key_hash()),
                    &signer.secret.key(),
                )
                .unwrap();
            })
            .with_app_config(AppConfig {
                addresses: AppAddresses {
                    account_factory,
                    // the other addresses don't matter
                    ..Default::default()
                },
                collateral_powers: Default::default(),
                ..Default::default()
            })
            .unwrap();

        let mut mock_ctx = MockContext::default()
            .with_chain_id("dango-1")
            .with_querier(mock_querier)
            .with_mode(AuthMode::Finalize);

        authenticate_tx(mock_ctx.as_auth(), tx, None).should_succeed();
    }

    #[test]
    fn sign_eip712_transaction_works() {
        let username = Username::from_str("alice").unwrap();
        let address = Addr::mock(0);
        let nonce = 0;
        let account_factory = Addr::mock(1);

        let mut signer = SingleSigner::new(username.as_ref(), address, Eip712::new_random())
            .unwrap()
            .with_nonce(nonce);

        let tx = signer
            .sign_transaction(
                NonEmpty::new_unchecked(vec![
                    Message::transfer(Addr::mock(2), Coins::one("uatom", 100).unwrap()).unwrap(),
                    Message::transfer(Addr::mock(3), Coins::one("uosmo", 500).unwrap()).unwrap(),
                ]),
                "dango-1",
                100_000_000,
            )
            .unwrap();

        let mock_querier = MockQuerier::new()
            .with_raw_contract_storage(account_factory, |storage| {
                ACCOUNTS_BY_USER
                    .insert(storage, (&username, address))
                    .unwrap();
                KEYS.save(
                    storage,
                    (&username, signer.secret.key_hash()),
                    &signer.secret.key(),
                )
                .unwrap();
            })
            .with_app_config(AppConfig {
                addresses: AppAddresses {
                    account_factory,
                    // the other addresses don't matter
                    ..Default::default()
                },
                collateral_powers: Default::default(),
                ..Default::default()
            })
            .unwrap();

        let mut mock_ctx = MockContext::default()
            .with_chain_id("dango-1")
            .with_querier(mock_querier)
            .with_mode(AuthMode::Finalize);

        authenticate_tx(mock_ctx.as_auth(), tx, None).should_succeed();
    }

    #[ignore = "Disabling this since it doesn't test anything"]
    #[test]
    fn unsigned_tx() {
        let username = Username::from_str("owner").unwrap();
        let address = Addr::from_str("0x33361de42571d6aa20c37daa6da4b5ab67bfaad9").unwrap();

        let signer = SingleSigner::new(username.as_ref(), address, Secp256k1::new_random())
            .unwrap()
            .with_nonce(1);

        let tx = signer
            .unsigned_transaction(
                NonEmpty::new_unchecked(vec![
                    Message::transfer(
                        Addr::from_str("0x01bba610cbbfe9df0c99b8862f3ad41b2f646553").unwrap(),
                        Coins::one("hyp/all/btc", 100).unwrap(),
                    )
                    .unwrap(),
                ]),
                "dev-6",
            )
            .unwrap();

        println!("{}", tx.to_json_string_pretty().unwrap());
    }
}
