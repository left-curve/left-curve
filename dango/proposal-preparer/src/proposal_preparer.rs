use {
    crate::pyth_handler::PythHandler,
    dango_types::oracle::ExecuteMsg,
    grug::{Addr, Coins, Json, JsonSerExt, Message, NonEmpty, StdError, Tx},
    prost::bytes::Bytes,
    pyth_client::{PythClient, PythClientCache, PythClientTrait},
    pyth_types::{PythLazerSubscriptionDetails, constants::LAZER_ENDPOINTS_TEST},
    reqwest::IntoUrl,
    std::{fmt::Debug, sync::Mutex},
    tracing::warn,
};
#[cfg(feature = "metrics")]
use {
    metrics::{describe_histogram, histogram},
    std::time::Instant,
};

const GAS_LIMIT: u64 = 50_000_000;

pub struct ProposalPreparer<P>
where
    P: PythClientTrait,
{
    // `Option` to be able to not clone the `PythHandler`.
    client_and_oracle: Option<(Mutex<PythHandler<P>>, Addr)>,
}

impl<P> Clone for ProposalPreparer<P>
where
    P: PythClientTrait,
{
    fn clone(&self) -> Self {
        Self {
            client_and_oracle: None,
        }
    }
}

impl ProposalPreparer<PythClient> {
    pub fn new<U, T>(
        oracle: Option<Addr>,
        endpoints: Vec<U>,
        access_token: T,
        ids: Vec<PythLazerSubscriptionDetails>,
    ) -> Self
    where
        U: IntoUrl,
        T: ToString,
    {
        #[cfg(feature = "metrics")]
        init_metrics();

        let mut client_and_oracle = None;

        if oracle.is_none() {
            warn!("Oracle address is not provided! Oracle feeding is disabled");
        } else if endpoints.is_empty() {
            warn!("Pyth Lazer endpoints not provided! Oracle feeding is disabled");
        } else if access_token.to_string().is_empty() {
            warn!("Pyth Lazer access token is empty! Oracle feeding is disabled");
        } else if ids.is_empty() {
            warn!("Pyth Lazer subscription details is empty! Oracle feeding is disabled");
        } else {
            let mut client = PythHandler::new(
                NonEmpty::new_unchecked(endpoints),
                access_token,
                NonEmpty::new_unchecked(ids),
            );

            client.connect_stream();

            client_and_oracle = Some((Mutex::new(client), oracle.unwrap())); // unwrap is safe because we already checked it's not `None`.
        }

        Self { client_and_oracle }
    }
}

impl ProposalPreparer<PythClientCache> {
    pub fn new_with_cache(oracle: Addr, ids: NonEmpty<Vec<PythLazerSubscriptionDetails>>) -> Self {
        #[cfg(feature = "metrics")]
        init_metrics();

        let client = PythHandler::new_with_cache(
            NonEmpty::new(LAZER_ENDPOINTS_TEST).unwrap(),
            "lazer_token",
            ids,
        );

        Self {
            client_and_oracle: Some((Mutex::new(client), oracle)),
        }
    }
}

impl<P> grug_app::ProposalPreparer for ProposalPreparer<P>
where
    P: PythClientTrait + Send + 'static,
    P::Error: Debug,
{
    type Error = StdError;

    fn prepare_proposal(
        &self,
        mut txs: Vec<Bytes>,
        _max_tx_bytes: usize,
    ) -> Result<Vec<Bytes>, Self::Error> {
        #[cfg(feature = "metrics")]
        let start = Instant::now();

        // Create the Pyth handler and start streaming.
        let Some((mutex, oracle)) = &self.client_and_oracle else {
            // Do nothing if the Pyth handler is uninitialized.
            return Ok(txs);
        };

        let pyth_handler = mutex.lock().expect("pyth handler poisoned");

        // Retrieve the PriceUpdate. Return if there are no new prices to feed.
        let Some(price_update) = pyth_handler.fetch_latest_price_update() else {
            return Ok(txs);
        };

        // Build the tx.
        let tx = Tx {
            sender: *oracle,
            gas_limit: GAS_LIMIT,
            msgs: NonEmpty::new_unchecked(vec![Message::execute(
                *oracle,
                &ExecuteMsg::FeedPrices(price_update),
                Coins::new(),
            )?]),
            data: Json::null(),
            credential: Json::null(),
        };

        txs.insert(0, tx.to_json_vec()?.into());

        #[cfg(feature = "metrics")]
        histogram!("proposal_preparer.prepare_proposal.duration")
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
