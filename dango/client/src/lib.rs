use {
    bip32::{Language, Mnemonic, PrivateKey, XPrv},
    dango_types::{
        account::spot,
        account_factory::Username,
        auth::{Credential, Key, Metadata, SignDoc, Signature, StandardCredential},
    },
    grug::{
        Addr, Addressable, ByteArray, Client, Defined, Hash256, HashExt, Inner, JsonSerExt,
        LengthBounded, MaybeDefined, Message, Signer, StdResult, Tx, Undefined,
    },
    grug_crypto::Identity256,
    k256::ecdsa::{self, signature::DigestSigner, SigningKey},
    rand::rngs::OsRng,
    std::str::FromStr,
};

pub const DEFAULT_DERIVATION_PATH: &str = "m/44'/60'/0'/0/0";

/// Utility for signing transactions in the format by Dango's single-signature
/// accounts, i.e. spot and margin accounts.
pub struct SingleSigner<T>
where
    T: MaybeDefined<u32>,
{
    pub username: Username,
    pub address: Addr,
    pub key: Key,
    pub key_hash: Hash256,
    sk: SigningKey,
    nonce: T,
}

impl SingleSigner<Undefined<u32>> {
    pub fn new(username: &str, address: Addr, sk: SigningKey) -> anyhow::Result<Self> {
        let username = Username::from_str(username)?;
        let pk = sk.public_key().to_encoded_point(true).to_bytes();

        Ok(Self {
            username,
            address,
            key: Key::Secp256k1(ByteArray::from_inner(pk.as_ref().try_into().unwrap())),
            key_hash: pk.hash256(),
            sk,
            nonce: Undefined::new(),
        })
    }

    pub fn new_random(username: &str, address: Addr) -> anyhow::Result<Self> {
        Self::new(username, address, SigningKey::random(&mut OsRng))
    }

    pub fn from_private_key(username: &str, address: Addr, key: [u8; 32]) -> anyhow::Result<Self> {
        Self::new(username, address, SigningKey::from_bytes(&key.into())?)
    }

    pub fn from_mnemonic(
        username: &str,
        address: Addr,
        mnemonic: &str,
        derivation_path: Option<&str>,
    ) -> anyhow::Result<Self> {
        let mnemonic = Mnemonic::new(mnemonic, Language::English)?;
        let seed = mnemonic.to_seed("");
        let path = derivation_path.unwrap_or(DEFAULT_DERIVATION_PATH).parse()?;
        let sk = SigningKey::from(XPrv::derive_from_path(seed, &path)?);

        Self::new(username, address, sk)
    }

    pub fn with_nonce(self, nonce: u32) -> SingleSigner<Defined<u32>> {
        SingleSigner {
            username: self.username,
            address: self.address,
            key: self.key,
            key_hash: self.key_hash,
            sk: self.sk,
            nonce: Defined::new(nonce),
        }
    }

    pub async fn query_nonce(self, client: &Client) -> anyhow::Result<SingleSigner<Defined<u32>>> {
        let nonce = client
            .query_wasm_smart(self.address, &spot::QueryMsg::Nonce {}, None)
            .await?;

        Ok(SingleSigner {
            username: self.username,
            address: self.address,
            key: self.key,
            key_hash: self.key_hash,
            sk: self.sk,
            nonce: Defined::new(nonce),
        })
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
    fn sign_transaction(
        &mut self,
        msgs: Vec<Message>, // TODO: the method should take a `LengthBounded`
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
            messages: LengthBounded::new_unchecked(msgs.clone()), // TODO
            data: metadata.clone(),
        }
        .to_json_vec()?
        .hash256()
        .into_inner();

        let signature: ecdsa::Signature = self.sk.sign_digest(Identity256::from(sign_doc));
        let signature = signature.to_bytes();

        let credential = Credential::Standard(StandardCredential {
            key_hash: self.key_hash,
            signature: Signature::Secp256k1(ByteArray::from_inner(signature.into())),
        });

        Ok(Tx {
            sender: self.address,
            gas_limit,
            msgs: LengthBounded::new_unchecked(msgs.clone()), // TODO
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
        dango_auth::{authenticate_tx, NEXT_NONCE},
        dango_types::config::{AppAddresses, AppConfig},
        grug::{AuthMode, Coins, MockContext, MockQuerier, MockStorage, ResultExt, Storage},
    };

    #[test]
    fn sign_transaction_works() {
        let username = Username::from_str("alice").unwrap();
        let address = Addr::mock(0);
        let nonce = 456;
        let account_factory = Addr::mock(1);

        let mut signer = SingleSigner::new_random(username.as_ref(), address)
            .unwrap()
            .with_nonce(nonce);

        let tx = signer
            .sign_transaction(
                vec![
                    Message::transfer(Addr::mock(2), Coins::one("uatom", 100).unwrap()).unwrap(),
                    Message::transfer(Addr::mock(3), Coins::one("uosmo", 500).unwrap()).unwrap(),
                ],
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
                    ibc_transfer: Addr::mock(0),
                    lending: Addr::mock(0),
                    oracle: Addr::mock(0),
                },
                collateral_powers: Default::default(),
            })
            .unwrap();

        let mut mock_ctx = MockContext::default()
            .with_storage({
                let mut storage = MockStorage::new();
                storage.write(NEXT_NONCE.storage_key(), nonce.to_le_bytes().as_ref());
                storage
            })
            .with_querier(mock_querier)
            .with_mode(AuthMode::Finalize);

        authenticate_tx(mock_ctx.as_auth(), tx, None, None).should_succeed();
    }
}
