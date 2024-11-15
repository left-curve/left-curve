use {
    dango_types::{
        config::AppConfig,
        oracle::{ExecuteMsg, PriceSource, QueryPriceSourcesRequest},
    },
    grug::{Binary, Coins, Json, JsonSerExt, Message, NonEmpty, QuerierWrapper, StdError, Tx},
    grug_app::Shared,
    prost::bytes::Bytes,
    std::{
        thread::{self, JoinHandle},
        time,
    },
    thiserror::Error,
    tracing::error,
};

const PYTH_URL: &str = "https://hermes.pyth.network";
const REQUEST_TIMEOUT: time::Duration = time::Duration::from_millis(500);
const THREAD_SLEEP: time::Duration = time::Duration::from_millis(500);
const GAS_LIMIT: u64 = 50_000_000;

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

pub struct ProposalPreparer {
    params: Shared<Vec<(String, String)>>,
    latest_vaas: Shared<Vec<Binary>>,
    // Option since we don't want to clone the thread handle.
    // Store the thread to keep it alive.
    _handle: Option<JoinHandle<()>>,
}

impl Clone for ProposalPreparer {
    fn clone(&self) -> Self {
        Self {
            params: self.params.clone(),
            latest_vaas: self.latest_vaas.clone(),
            _handle: None,
        }
    }
}

impl Default for ProposalPreparer {
    fn default() -> Self {
        Self::new()
    }
}

impl ProposalPreparer {
    pub fn new() -> Self {
        let params = Shared::new(Vec::new());
        let thread_params = params.clone();
        let latest_vaas = Shared::new(Vec::new());
        let thread_latest_vaas = latest_vaas.clone();

        let _handle = thread::spawn(move || {
            let update_func = || -> Result<(), ProposerError> {
                // Copy the params to unlock the mutex.
                let params = thread_params.read_access().clone();
                if params.is_empty() {
                    return Ok(());
                }

                // Retrieve vaas from pyth node.
                let vaas = reqwest::blocking::Client::builder()
                    .timeout(REQUEST_TIMEOUT)
                    .build()?
                    .get(format!("{PYTH_URL}/api/latest_vaas"))
                    .query(&params)
                    .send()?
                    .json::<Vec<Binary>>()?;

                tracing::info!(
                    "Prepare proposal: fetched latest vaas - len: {}",
                    vaas.len()
                );

                // Update the prices.
                thread_latest_vaas.write_with(|mut latest_vaas| {
                    *latest_vaas = vaas;
                });

                Ok(())
            };

            loop {
                // Update the vaas.
                update_func().unwrap_or_else(|err| {
                    error!(err = err.to_string(), "Failed to update the latest vaas")
                });

                // Wait `THREAD_SLEEP` before the next iteration.
                thread::sleep(THREAD_SLEEP);
            }
        });

        Self {
            params,
            latest_vaas,
            _handle: Some(_handle),
        }
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
                    Some(("ids[]".to_string(), id.to_string()))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        // Write the params to the shared memory.
        self.params.write_with(|mut params_ref| {
            *params_ref = params;
        });

        // Retreive the vaas from the shared memory.
        // Consuming the vaas to avoid feeding the same prices multiple times.
        let vaas = self.latest_vaas.write_with(|mut prices_lock| {
            let prices = prices_lock.clone();
            *prices_lock = vec![];
            prices
        });

        // Return if there are no vaas to feed.
        if vaas.is_empty() {
            return Ok(txs);
        }

        // Build the tx.
        let tx = Tx {
            sender: cfg.addresses.oracle,
            gas_limit: GAS_LIMIT,
            msgs: NonEmpty::new_unchecked(vec![Message::execute(
                cfg.addresses.oracle,
                &ExecuteMsg::FeedPrices(NonEmpty::new(vaas)?),
                Coins::new(),
            )?]),
            data: Json::null(),
            credential: Json::null(),
        };

        txs.insert(0, tx.to_json_vec()?.into());

        Ok(txs)
    }
}
