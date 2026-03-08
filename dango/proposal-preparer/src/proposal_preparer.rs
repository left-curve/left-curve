use {
    crate::{QueryPythId, pyth_handler::PythHandler},
    dango_types::{config::AppConfig, oracle::ExecuteMsg},
    grug::{
        Coins, Json, JsonSerExt, Lengthy, Message, NonEmpty, QuerierExt, QuerierWrapper, StdError,
        Tx,
    },
    prost::bytes::Bytes,
    pyth_client::{PythClient, PythClientCache, PythClientTrait},
    pyth_types::constants::LAZER_ENDPOINTS_TEST,
    reqwest::IntoUrl,
    std::{fmt::Debug, sync::Mutex},
    tracing::{error, warn},
};
#[cfg(feature = "metrics")]
use {
    metrics::{describe_histogram, histogram},
    std::time::Instant,
};

const GAS_LIMIT: u64 = 50_000_000;

pub struct ProposalPreparer<P>
where
    P: PythClientTrait + QueryPythId,
{
    // `Option` to be able to not clone the `PythHandler`.
    pyth_handler: Option<Mutex<PythHandler<P>>>,
}

impl<P> Clone for ProposalPreparer<P>
where
    P: PythClientTrait + QueryPythId,
{
    fn clone(&self) -> Self {
        Self { pyth_handler: None }
    }
}

impl ProposalPreparer<PythClient> {
    pub fn new<V, U, T>(endpoints: V, access_token: T) -> Self
    where
        V: IntoIterator<Item = U> + Lengthy,
        U: IntoUrl,
        T: ToString,
    {
        #[cfg(feature = "metrics")]
        init_metrics();

        let mut client = None;

        if access_token.to_string().is_empty() {
            warn!("Pyth Lazer access token is empty! Oracle feeding is disabled");
        } else if endpoints.length() == 0 {
            warn!("Pyth Lazer endpoints not provided! Oracle feeding is disabled");
        } else {
            match NonEmpty::new(endpoints) {
                Ok(non_empty_endpoints) => {
                    match PythHandler::new(non_empty_endpoints, access_token) {
                        Ok(handler) => {
                            client = Some(Mutex::new(handler));
                        },
                        Err(err) => {
                            warn!(error = %err, "failed to initialize Pyth handler, oracle feeding is disabled");
                        },
                    }
                },
                Err(err) => {
                    warn!(error = %err, "invalid Pyth Lazer endpoints, oracle feeding is disabled");
                },
            };
        }

        Self {
            pyth_handler: client,
        }
    }
}

impl ProposalPreparer<PythClientCache> {
    pub fn new_with_cache() -> Self {
        #[cfg(feature = "metrics")]
        init_metrics();

        let maybe_client = match NonEmpty::new(LAZER_ENDPOINTS_TEST) {
            Ok(endpoints) => match PythHandler::new_with_cache(endpoints, "lazer_token") {
                Ok(handler) => Some(Mutex::new(handler)),
                Err(err) => {
                    warn!(error = %err, "failed to initialize cached Pyth handler, oracle feeding is disabled");
                    None
                },
            },
            Err(err) => {
                warn!(error = %err, "invalid cached Pyth endpoints, oracle feeding is disabled");
                None
            },
        };

        Self {
            pyth_handler: maybe_client,
        }
    }
}

// Ensure background streaming threads are stopped when the preparer is dropped.
impl<P> Drop for ProposalPreparer<P>
where
    P: PythClientTrait + QueryPythId,
{
    fn drop(&mut self) {
        if let Some(handler) = &self.pyth_handler
            && let Ok(mut h) = handler.lock()
        {
            h.close_stream();
        }
    }
}

impl<P> grug_app::ProposalPreparer for ProposalPreparer<P>
where
    P: PythClientTrait + QueryPythId + Send + 'static,
    P::Error: Debug,
{
    type Error = StdError;

    fn prepare_proposal(
        &self,
        querier: QuerierWrapper,
        mut txs: Vec<Bytes>,
        _max_tx_bytes: usize,
    ) -> Result<Vec<Bytes>, Self::Error> {
        #[cfg(feature = "metrics")]
        let start = Instant::now();

        let cfg: AppConfig = querier.query_app_config()?;

        // Check if the PythHandler is initialized.
        let Some(handler) = self.pyth_handler.as_ref() else {
            return Ok(txs);
        };

        // Should we find a way to start and connect the PythClientPPHandler at startup?
        // How to know which ids should be used?
        let mut pyth_handler = match handler.lock() {
            Ok(handler) => handler,
            Err(err) => {
                error!(error = %err, "failed to lock Pyth handler");
                return Ok(txs);
            },
        };

        // Update the Pyth stream if the PythIds in the oracle have changed.
        if let Err(err) = pyth_handler.update_stream(querier, cfg.addresses.oracle) {
            error!("Failed to update Pyth stream: {:?}", err);
        }

        // Retrieve the PriceUpdate.
        let maybe_price_update = pyth_handler.fetch_latest_price_update();

        // Return if there are no new prices to feed.
        let Some(price_update) = maybe_price_update else {
            return Ok(txs);
        };

        // Build the tx.
        let tx = Tx {
            sender: cfg.addresses.oracle,
            gas_limit: GAS_LIMIT,
            msgs: NonEmpty::new_unchecked(vec![Message::execute(
                cfg.addresses.oracle,
                &ExecuteMsg::FeedPrices(price_update),
                Coins::new(),
            )?]),
            data: Json::null(),
            credential: Json::null(),
        };

        txs.insert(0, tx.to_json_vec()?.into());

        #[cfg(feature = "metrics")]
        histogram!("proposal_preparer.prepare_proposal.duration",)
            .record(start.elapsed().as_secs_f64());

        Ok(txs)
    }
}

#[cfg(feature = "metrics")]
pub fn init_metrics() {
    describe_histogram!(
        "proposal_preparer.prepare_proposal.duration",
        "Duration of the `prepare_proposal` method in seconds",
    );
}
