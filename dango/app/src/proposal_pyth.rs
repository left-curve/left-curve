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
    grug_app::ProposalPreparer,
    k256::ecdsa::{signature::Signer, Signature, SigningKey, VerifyingKey},
    prost::bytes::Bytes,
    std::{ops::Deref, str::FromStr, time},
    thiserror::Error,
};

const PYTH_URL: &str = "https://hermes.pyth.network";
const DEFAULT_TIMEOUT: time::Duration = time::Duration::from_millis(500);
const GAS_LIMIT: u64 = 5_000_000;

#[derive(Debug, Error)]
pub enum PythError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    #[error(transparent)]
    Signature(#[from] k256::ecdsa::Error),
}

impl From<PythError> for grug_app::AppError {
    fn from(value: PythError) -> Self {
        grug_app::AppError::PrepareProposal(value.to_string())
    }
}

#[derive(Clone)]
pub struct PythProposalPreparer {
    chain_id: String,
    feeder_addr: Addr,
    feeder_sk: SigningKey,
    key_hash: Hash160,
    username: Username,
}

impl PythProposalPreparer {
    pub fn new(
        chain_id: String,
        feeder_addr: Addr,
        feeder_sk: &[u8],
        username: String,
    ) -> Result<Self, PythError> {
        let feeder_sk = SigningKey::from_slice(feeder_sk)?;
        let key_hash = VerifyingKey::from(&feeder_sk)
            .to_sec1_bytes()
            .deref()
            .hash160();
        let username = Username::from_str(&username)?;

        Ok(Self {
            chain_id,
            feeder_addr,
            feeder_sk,
            key_hash,
            username,
        })
    }

    pub fn sign_tx(&self, sequence: u32, messages: Vec<Message>) -> Result<Tx, PythError> {
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

impl ProposalPreparer for PythProposalPreparer {
    type Error = PythError;

    fn prepare_proposal(
        &self,
        querier: QuerierWrapper,
        mut txs: Vec<Bytes>,
        _max_tx_bytes: usize,
    ) -> Result<Vec<Bytes>, Self::Error> {
        let oracle = querier.query_app_config(ORACLE_KEY)?;

        let mut params = vec![];
        let mut start_after = None;

        // Keep collecting price ids from oracle contract until there are no more
        loop {
            let price_ids = querier.query_wasm_smart(oracle, QueryPriceSourcesRequest {
                start_after,
                limit: None,
            })?;

            if let Some((key, _)) = price_ids.last_key_value() {
                start_after = Some(key.clone());
            } else {
                break;
            }

            for price_id in price_ids.into_values() {
                #[allow(irrefutable_let_patterns)]
                // For now there is only Pyth as PriceSource, but there could be more
                if let PriceSource::Pyth { id, .. } = price_id {
                    params.push(("ids[]", id.to_string()));
                }
            }
        }

        // Retrive prices from pyth node
        let prices = reqwest::blocking::Client::builder()
            .timeout(DEFAULT_TIMEOUT)
            .build()?
            .get(format!("{PYTH_URL}/api/latest_vaas"))
            .query(&params)
            .send()?
            .json::<Vec<Binary>>()?;

        // build the tx
        let sequence = querier.query_wasm_smart(self.feeder_addr, QuerySequenceRequest {})?;

        let msgs = vec![Message::execute(
            oracle,
            &ExecuteMsg::FeedPrices(prices),
            Coins::new(),
        )?];

        let tx = self.sign_tx(sequence, msgs)?;

        txs.insert(0, tx.to_json_vec()?.into());

        Ok(txs)
    }
}