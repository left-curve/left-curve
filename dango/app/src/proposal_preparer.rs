use {
    dango_types::{
        account::multi::QuerySequenceRequest,
        account_factory::Username,
        auth::{Credential, Metadata, SignDoc},
        config::ORACLE_KEY,
        oracle::{ExecuteMsg, PriceSource, QueryPriceSourcesRequest},
    },
    grug::{
        Addr, Binary, Coins, Hash160, HashExt, JsonSerExt, Message, QuerierWrapper, StdError, Tx,
    },
    grug_app::Shared,
    k256::ecdsa::{signature::Signer, Signature, SigningKey, VerifyingKey},
    prost::bytes::Bytes,
    std::{
        ops::Deref,
        str::FromStr,
        thread::{self, JoinHandle},
        time,
    },
    thiserror::Error,
    tracing::error,
};

const PYTH_URL: &str = "https://hermes.pyth.network";
const DEFAULT_TIMEOUT: time::Duration = time::Duration::from_millis(500);
const GAS_LIMIT: u64 = 5_000_000;

#[derive(Debug, Error)]
pub enum ProposerError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    #[error(transparent)]
    Signature(#[from] k256::ecdsa::Error),
}

impl From<ProposerError> for grug_app::AppError {
    fn from(value: ProposerError) -> Self {
        grug_app::AppError::PrepareProposal(value.to_string())
    }
}

pub struct ProposalPreparer {
    chain_id: String,
    feeder_addr: Addr,
    feeder_sk: SigningKey,
    key_hash: Hash160,
    username: Username,

    _params: Shared<Vec<(String, String)>>,
    _prices: Shared<Vec<Binary>>,
    _handle: Option<JoinHandle<()>>,
}

impl Clone for ProposalPreparer {
    fn clone(&self) -> Self {
        Self {
            chain_id: self.chain_id.clone(),
            feeder_addr: self.feeder_addr,
            feeder_sk: self.feeder_sk.clone(),
            key_hash: self.key_hash,
            username: self.username.clone(),
            _params: self._params.clone(),
            _prices: self._prices.clone(),
            _handle: None,
        }
    }
}

impl ProposalPreparer {
    pub fn new(
        chain_id: String,
        feeder_addr: Addr,
        feeder_sk: &[u8],
        username: String,
    ) -> Result<Self, ProposerError> {
        let feeder_sk = SigningKey::from_slice(feeder_sk)?;
        let key_hash = VerifyingKey::from(&feeder_sk)
            .to_sec1_bytes()
            .deref()
            .hash160();
        let username = Username::from_str(&username)?;

        let _params = Shared::new(Vec::new());
        let thread_params = _params.clone();
        let _prices = Shared::new(Vec::new());
        let thread_prices = _prices.clone();
        let _handle = thread::spawn(move || {
            let update_func = || -> Result<(), ProposerError> {
                // Copy the params to unlock the mutex.
                let params = thread_params.read_access().clone();
                if params.is_empty() {
                    return Ok(());
                }

                // Retrieve vaas from pyth node.
                let vaas = reqwest::blocking::Client::builder()
                    .timeout(DEFAULT_TIMEOUT)
                    .build()?
                    .get(format!("{PYTH_URL}/api/latest_vaas"))
                    .query(&params)
                    .send()?
                    .json::<Vec<Binary>>()?;

                // Update the prices.
                thread_prices.write_with(|mut prices| {
                    *prices = vaas;
                });
                Ok(())
            };

            loop {
                // Update the prices.
                update_func().unwrap_or_else(|err| {
                    error!(
                        err = err.to_string(),
                        "Failed to prepare proposal! Falling back to naive preparer."
                    )
                });

                // Wait 500 ms before the next iteration.
                thread::sleep(time::Duration::from_millis(500));
            }
        });

        Ok(Self {
            chain_id,
            feeder_addr,
            feeder_sk,
            key_hash,
            username,
            _params,
            _prices,
            _handle: Some(_handle),
        })
    }

    pub fn sign_tx(&self, sequence: u32, messages: Vec<Message>) -> Result<Tx, ProposerError> {
        let sign_bytes = SignDoc {
            sender: self.feeder_addr,
            messages: messages.clone(),
            chain_id: self.chain_id.clone(),
            sequence,
        }
        .to_json_vec()?;

        let signature: Signature = self.feeder_sk.sign(&sign_bytes);

        let data = Metadata {
            username: self.username.clone(),
            key_hash: self.key_hash,
            sequence,
        };

        let credential = Credential::Secp256k1(signature.to_bytes().to_vec().try_into()?);

        Ok(Tx {
            sender: self.feeder_addr,
            gas_limit: GAS_LIMIT,
            msgs: messages,
            data: data.to_json_value()?,
            credential: credential.to_json_value()?,
        })
    }
}

impl grug_app::ProposalPreparer for ProposalPreparer {
    type Error = ProposerError;

    fn prepare_proposal(
        &self,
        querier: QuerierWrapper,
        mut txs: Vec<Bytes>,
        _max_tx_bytes: usize,
    ) -> Result<Vec<Bytes>, Self::Error> {
        let oracle = querier.query_app_config(ORACLE_KEY)?;

        // Retrieve the price ids from the oracle and prepare the query params.
        let params = querier
            .query_wasm_smart(oracle, QueryPriceSourcesRequest {
                start_after: None,
                limit: Some(u32::MAX),
            })?
            .into_values()
            .filter_map(|price_id| {
                // For now there is only Pyth as PriceSource, but there could be more.
                #[allow(irrefutable_let_patterns)]
                if let PriceSource::Pyth { id, .. } = price_id {
                    Some(("ids[]".to_string(), id.to_string()))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        // Write the params to the shared memory.
        self._params.write_with(|mut params_ref| {
            *params_ref = params;
        });

        // Retreive the prices from the shared memory.
        // Consuming the vaas is necessary to avoid feeding the same prices multiple times
        // (it would not be an error but it will consume fee for tx).
        let vaas = self._prices.write_with(|mut prices_lock| {
            let prices = prices_lock.clone();
            *prices_lock = vec![];
            prices
        });

        // Return if there are no prices to feed.
        if vaas.is_empty() {
            return Ok(txs);
        }

        // Build the tx.
        let sequence = querier.query_wasm_smart(self.feeder_addr, QuerySequenceRequest {})?;

        let msgs = vec![Message::execute(
            oracle,
            &ExecuteMsg::FeedPrices(vaas),
            Coins::new(),
        )?];

        let tx = self.sign_tx(sequence, msgs)?;

        txs.insert(0, tx.to_json_vec()?.into());

        Ok(txs)
    }
}
