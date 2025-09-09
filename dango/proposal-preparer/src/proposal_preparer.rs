use {
    crate::{QueryPythId, pyth_handler::PythHandler},
    dango_types::{config::AppConfig, oracle::ExecuteMsg},
    grug::{
        Coins, Json, JsonSerExt, Lengthy, Message, NonEmpty, QuerierExt, QuerierWrapper, StdError,
        Tx,
    },
    prost::bytes::Bytes,
    pyth_client::{PythClientCore, PythClientCoreCache, PythClientTrait},
    pyth_lazer::{PythClientLazer, PythClientLazerCache},
    pyth_types::constants::{LAZER_ACCESS_TOKEN_TEST, LAZER_ENDPOINTS_TEST, PYTH_URL},
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
    P: PythClientTrait,
{
    // `Option` to be able to not clone the `PythHandler`.
    pyth_handler: Option<Mutex<PythHandler<P>>>,
}

impl<P> Clone for ProposalPreparer<P>
where
    P: PythClientTrait,
{
    fn clone(&self) -> Self {
        Self { pyth_handler: None }
    }
}

impl ProposalPreparer<PythClientCore> {
    pub fn new() -> Self {
        #[cfg(feature = "metrics")]
        init_metrics();

        let client = PythHandler::new_with_core(PYTH_URL);

        Self {
            pyth_handler: Some(Mutex::new(client)),
        }
    }
}

impl Default for ProposalPreparer<PythClientCore> {
    fn default() -> Self {
        Self::new()
    }
}

impl ProposalPreparer<PythClientCoreCache> {
    pub fn new_with_cache() -> Self {
        #[cfg(feature = "metrics")]
        init_metrics();

        let client = PythHandler::new_with_core_cache(PYTH_URL);

        Self {
            pyth_handler: Some(Mutex::new(client)),
        }
    }
}

impl ProposalPreparer<PythClientLazer> {
    pub fn new_with_lazer<V, U, T>(endpoints: V, access_token: T) -> Self
    where
        V: IntoIterator<Item = U> + Lengthy,
        U: IntoUrl,
        T: ToString,
    {
        #[cfg(feature = "metrics")]
        init_metrics();

        let mut client = None;

        if access_token.to_string().is_empty() {
            warn!("Access token for Pyth Lazer is empty, oracle feeding will be disabled");
        } else {
            if endpoints.length() == 0 {
                warn!("Endpoints for Pyth Lazer not provided, oracle feeding will be disabled");
            } else {
                client = Some(Mutex::new(PythHandler::new_with_lazer(
                    NonEmpty::new(endpoints).unwrap(),
                    access_token,
                )))
            }
        }

        Self {
            pyth_handler: client,
        }
    }
}

impl ProposalPreparer<PythClientLazerCache> {
    pub fn new_with_lazer_cache() -> Self {
        #[cfg(feature = "metrics")]
        init_metrics();

        let client = PythHandler::new_with_lazer_cache(
            NonEmpty::new(LAZER_ENDPOINTS_TEST).unwrap(),
            LAZER_ACCESS_TOKEN_TEST,
        );

        Self {
            pyth_handler: Some(Mutex::new(client)),
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
        if self.pyth_handler.is_none() {
            return Ok(txs);
        }

        // Should we find a way to start and connect the PythClientPPHandler at startup?
        // How to know which ids should be used?
        let mut pyth_handler = self.pyth_handler.as_ref().unwrap().lock().unwrap();

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
        "Duration of the prepare_proposal method in seconds",
    );
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod test {
    use {grug::JsonDeExt, pyth_types::LatestVaaResponse};

    #[test]
    fn deserializing_pyth_response() {
        r#"{
          "binary": {
            "encoding": "base64",
            "data": [
              "UE5BVQEAAAADuAEAAAAEDQBkr0PLb+gk8uvpb1vCCnSzrkNBWuAKD+4/oHA1HhL2rywRRNl4NEsUMyNDHVVFJz7sb2TqUXbIVKSDR+cXZk12AQHIF5lSaXo4Br8HN9I8FxSKI+k39d0G/hfGGcg42L1lmhol2f3hJRw32z9e9ktuCvOIClAe0U0t8hQk2meeHWReAARHzS7dEnqYLo5cM0ct+0lmMftM+SER9GP/Kr/l1nnUaRNff+2443LwCqOay1A0DSn6sOa6FO16w5mbgsNiUuMlAAhiIiNh1QIxaoKUydS3R0MnKoBkdt7ixtVCvK/GPi0PeC3goY+ZgaheHaVYt6lfjD0nwITz2bdYFZNq2SqO510VAAoHTixQaLHPPgi72kww0j5hOlJn11W1Rz8LAGGl0gk7/GZwEhiBUCuUCfTFwpHqX4UHJIXftR0SV1mS6UB3XLV6AAtAVczuOpFCCPiH9Sg9I0l1xitkQotpP8h+di11JJ3CVkm6vRU7zy1KrhYFsoV082IZTi0XN0Xdv5fWZZdg3Hx5AAzpW38X3a88bFIoys/jeflLAN+A9VeABd5HN9D6snOgI3o5FXftoZjNP0c8Xd/J8rTlUgBIOgsyGFh2vRjJbe6JAQ2+2P9NCAAhBnOYoRjASsA/XlI2FDEGK58ati8kz4vJGyO9B8O6sdZiKE70ieoDirBl8puIkYb0sPU8vov/xF1kAA4cPqmQw6zUazq7F8VE0FSe9C2KD3bD2drA/5rkFExyABw+4XL/4KGbA02YEvp4rGpQvBHKjyS7HhdfuWaspCtjAQ8IoVHbmdLHcnc1dJLgduCtMOPVB3+kpJZBLRfu962mRhG6zLmIr5ioO8/HSsDSQWLWGfi7u2z+g9Nw0CVVhmwmARBAmbiFQ3nI4Idyu9XHuY9r9FaQa1rAcdCuDEGCZPdbphqX7kckA6r95J01ERAqLHwNvZDzuTTgY00Qcuh4v7bfABELTI+OKdU8zctRIb+HJxqQJKeCuBc2+zFo0Tfjs7A8Ix4mszdY/c+1iKZjQLP8MvDekc1rPgjCbhJeZrvoz7xHABJoPDAMGFjauAjWB3g8EJHV+V+oTH5pYLXfZrXdUzzHxTj+JdMaJNIl7EN9MQ7hPzwKkkvwEKuHx661sLdMwGt1AWc3bGMAAAAAABrhAfrtrFhR4yubI7X5QRqMK6xKrj7U3XuBHdGnLqSqcQAAAAAFkcwYAUFVV1YAAAAAAAqelroAACcQy4n8eAYQTYIOJi7hMVMnT8+xofsBAFUA5i32yLSoX+GmfbRNwS3l2zMPesZrctxliv7fD0pBW0MAAAgFdR9DkgAAAAErfT7c////+AAAAABnN2xjAAAAAGc3bGMAAAgVwbgMwAAAAAErXavAC2svmTTo0dM2ChAXPyvygP0QyQvSR/BV5sMFociUhrsruHLNPmvHkOxzZMH4NQzepTk6Wc6cGiVU1RVKdqDYcPPsm+N2jsKYH5zp65jeHXG0h5pR53BajAgRmNmq7xzBP5e1vT6gv5CHEbv1QGAvbGXSReuOOj1LR29RgjJiM/A/PCysDvPuhI5r9QqT69mMvlraw2QdqlRGmTsq/1LlskABMC+bL64zMUlwLmo0N3kxXuS3Y906SY9J733py3EPMQMRrGD5C185kIK9iYlvyEOx5Tq0wvQeaB2/J7Q="
            ]
          }
        }"#
        .deserialize_json::<LatestVaaResponse>()
        .unwrap();
    }
}
