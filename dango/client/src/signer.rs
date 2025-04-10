use {
    crate::SigningKey,
    bip32::{Language, Mnemonic},
    dango_types::{
        account::spot,
        account_factory::Username,
        auth::{Credential, Key, Metadata, Nonce, SignDoc, Signature, StandardCredential},
    },
    grug::{
        Addr, Addressable, ByteArray, Defined, Hash256, HashExt, JsonSerExt, MaybeDefined, Message,
        NonEmpty, QueryClient, QueryClientExt, SignData, Signer, StdResult, Tx, Undefined,
        UnsignedTx,
    },
    std::str::FromStr,
};

pub const DEFAULT_DERIVATION_PATH: &str = "m/44'/60'/0'/0/0";

/// Utility for signing transactions in the format by Dango's single-signature
/// accounts, i.e. spot and margin accounts.
#[derive(Debug)]
pub struct SingleSigner<T>
where
    T: MaybeDefined<u32>,
{
    pub username: Username,
    pub address: Addr,
    pub key: Key,
    pub key_hash: Hash256,
    pub nonce: T,
    pub sk: SigningKey,
}

impl<T> SingleSigner<T>
where
    T: MaybeDefined<u32>,
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

impl SingleSigner<Undefined<u32>> {
    pub fn new(username: &str, address: Addr, sk: SigningKey) -> anyhow::Result<Self> {
        let username = Username::from_str(username)?;

        Ok(Self {
            username,
            address,
            key: Key::Secp256k1(ByteArray::from_inner(sk.public_key())),
            key_hash: sk.public_key().hash256(),
            nonce: Undefined::new(),
            sk,
        })
    }

    pub fn new_random(username: &str, address: Addr) -> anyhow::Result<Self> {
        Self::new(username, address, SigningKey::new_random())
    }

    pub fn from_private_key(username: &str, address: Addr, key: [u8; 32]) -> anyhow::Result<Self> {
        Self::new(username, address, SigningKey::from_bytes(key)?)
    }

    pub fn from_mnemonic(
        username: &str,
        address: Addr,
        mnemonic: &str,
        coin_type: usize,
    ) -> anyhow::Result<Self> {
        let mnemonic = Mnemonic::new(mnemonic, Language::English)?;
        let sk = SigningKey::from_mnemonic(&mnemonic, coin_type)?;

        Self::new(username, address, sk)
    }

    pub fn with_nonce(self, nonce: u32) -> SingleSigner<Defined<u32>> {
        SingleSigner {
            username: self.username,
            address: self.address,
            key: self.key,
            key_hash: self.key_hash,
            nonce: Defined::new(nonce),
            sk: self.sk,
        }
    }

    pub async fn query_nonce<C>(self, client: &C) -> anyhow::Result<SingleSigner<Defined<u32>>>
    where
        C: QueryClient,
        anyhow::Error: From<C::Error>,
    {
        let nonce = self.query_next_nonce(client).await?;

        Ok(SingleSigner {
            username: self.username,
            address: self.address,
            key: self.key,
            key_hash: self.key_hash,
            nonce: Defined::new(nonce),
            sk: self.sk,
        })
    }
}

impl SingleSigner<Defined<u32>> {
    pub async fn update_nonce<C>(&mut self, client: &C) -> anyhow::Result<()>
    where
        C: QueryClient,
        anyhow::Error: From<C::Error>,
    {
        let nonce = self.query_next_nonce(client).await?;

        self.nonce = Defined::new(nonce);

        Ok(())
    }
}

impl<T> Addressable for SingleSigner<T>
where
    T: MaybeDefined<u32>,
{
    fn address(&self) -> Addr {
        self.address
    }
}

impl Signer for SingleSigner<Defined<u32>> {
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
        let sign_data = sign_doc.to_sign_data()?;

        let credential = Credential::Standard(StandardCredential {
            key_hash: self.key_hash,
            signature: Signature::Secp256k1(self.sk.sign_digest(sign_data.into()).into()),
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

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_account_factory::{ACCOUNTS_BY_USER, KEYS},
        dango_auth::authenticate_tx,
        dango_types::config::{AppAddresses, AppConfig},
        grug::{AuthMode, Coins, MockContext, MockQuerier, ResultExt},
    };

    #[test]
    fn sign_transaction_works() {
        let username = Username::from_str("alice").unwrap();
        let address = Addr::mock(0);
        let nonce = 0;
        let account_factory = Addr::mock(1);

        let mut signer = SingleSigner::new_random(username.as_ref(), address)
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
                KEYS.save(storage, (&username, signer.key_hash), &signer.key)
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
    fn unsigned_tx() {
        let username = Username::from_str("owner").unwrap();
        let address = Addr::from_str("0x33361de42571d6aa20c37daa6da4b5ab67bfaad9").unwrap();

        let signer = SingleSigner::new_random(username.as_ref(), address)
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
