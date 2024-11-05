use {
    dango_types::oracle::{ExecuteMsg, PriceSource, QueryPriceSourcesRequest},
    grug::{Binary, Coins, Json, JsonSerExt, QuerierWrapper, StdError, Tx},
    grug_app::ProposalPreparer,
    grug_types::Message,
    prost::bytes::Bytes,
    thiserror::Error,
};

const PYTH_URL: &str = "https://hermes.pyth.network";

#[derive(Debug, Clone, Copy)]
pub struct PythProposalPreparer;

#[derive(Debug, Error)]
pub enum PythError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    #[error(transparent)]
    Error(#[from] anyhow::Error),
}

impl ProposalPreparer for PythProposalPreparer {
    type Error = PythError;

    fn prepare_proposal(
        &self,
        querier: QuerierWrapper,
        mut txs: Vec<Bytes>,
        _max_tx_bytes: usize,
    ) -> Result<Vec<Bytes>, Self::Error> {
        let oracle = querier.query_app_config("oracle")?;

        let mut params = vec![];
        let mut start_after = None;

        // Keep collecting price ids from oracle contract until there are no more
        loop {
            // let price_ids: BTreeMap<Denom, PriceSourceCollector> = BTreeMap::new();
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
                // For now there is only Pyth as PriceSourceCollector, but there could be more
                if let PriceSource::Pyth { id, .. } = price_id {
                    params.push(("ids[]", id.to_string()));
                }
            }
        }

        // Retrive prices from pyth node
        let prices = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_millis(500))
            .build()?
            .get(format!("{PYTH_URL}/api/latest_vaas"))
            .query(&params)
            .send()?
            .json::<Vec<Binary>>()?;

        // Push prices on chain
        let tx = Tx {
            sender: oracle,
            gas_limit: 1_000_000,
            msgs: vec![Message::execute(
                oracle,
                &ExecuteMsg::FeedPrices(prices),
                Coins::new(),
            )?],
            data: Json::null(),
            credential: Json::null(),
        }
        .to_json_vec()?
        .into();

        txs.insert(0, tx);

        Ok(txs)
    }
}
