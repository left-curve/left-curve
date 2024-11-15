use {
    dango_types::{
        config::AppConfig,
        oracle::{ExecuteMsg, PriceSource, QueryPriceSourcesRequest},
    },
    grug::{Binary, Coins, Json, JsonSerExt, Message, NonEmpty, QuerierWrapper, StdError, Tx},
    prost::bytes::Bytes,
    std::time,
    thiserror::Error,
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
}

impl From<ProposerError> for grug_app::AppError {
    fn from(value: ProposerError) -> Self {
        grug_app::AppError::PrepareProposal(value.to_string())
    }
}

#[derive(Clone)]
pub struct ProposalPreparer;

impl grug_app::ProposalPreparer for ProposalPreparer {
    type Error = ProposerError;

    fn prepare_proposal(
        &self,
        querier: QuerierWrapper,
        mut txs: Vec<Bytes>,
        _max_tx_bytes: usize,
    ) -> Result<Vec<Bytes>, Self::Error> {
        let cfg: AppConfig = querier.query_app_config()?;

        // Retrieve the price ids from the oracle and prepare the query params.
        // TODO: optimize this by using the raw WasmScan query.
        let params = querier
            .query_wasm_smart(cfg.addresses.oracle, QueryPriceSourcesRequest {
                start_after: None,
                limit: Some(u32::MAX),
            })?
            .into_values()
            .filter_map(|price_source| {
                // For now there is only Pyth as PriceSource, but there could be more.
                #[allow(irrefutable_let_patterns)]
                if let PriceSource::Pyth { id, .. } = price_source {
                    Some(("ids[]", id.to_string()))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        // Retrieve prices from pyth node.
        let prices = reqwest::blocking::Client::builder()
            .timeout(DEFAULT_TIMEOUT)
            .build()?
            .get(format!("{PYTH_URL}/api/latest_vaas"))
            .query(&params)
            .send()?
            .json::<Vec<Binary>>()?;

        // Build the tx.
        let tx = Tx {
            sender: cfg.addresses.oracle,
            gas_limit: GAS_LIMIT,
            msgs: NonEmpty::new_unchecked(vec![Message::execute(
                cfg.addresses.oracle,
                &ExecuteMsg::FeedPrices(NonEmpty::new(prices)?),
                Coins::new(),
            )?]),
            data: Json::null(),
            credential: Json::null(),
        };

        txs.insert(0, tx.to_json_vec()?.into());

        Ok(txs)
    }
}
