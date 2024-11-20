use {
    dango_types::{
        config::AppConfig,
        oracle::{ExecuteMsg, PriceSource, QueryPriceSourcesRequest},
    },
    grug::{Binary, Coins, Json, JsonSerExt, Message, NonEmpty, QuerierWrapper, StdError, Tx},
    grug_app::{AppError, Shared},
    prost::bytes::Bytes,
    std::{
        cmp::min,
        ops::Mul,
        thread::{self, JoinHandle},
        time::Duration,
    },
    thiserror::Error,
    tracing::{error, info},
};

const PYTH_URL: &str = "https://hermes.pyth.network";
const REQUEST_TIMEOUT: Duration = Duration::from_millis(1000);
const THREAD_SLEEP: Duration = Duration::from_millis(1000);
const THREAD_SLEEP_ON_FIRST_429: Duration = Duration::from_millis(5000);
const MAX_THREAD_SLEEP: Duration = Duration::from_secs(30);
const GAS_LIMIT: u64 = 50_000_000;

#[grug::derive(Serde)]
struct LatestVaaResponse {
    pub binary: LatestVaaBinaryResponse,
}

#[grug::derive(Serde)]
struct LatestVaaBinaryResponse {
    pub data: Vec<Binary>,
}

#[derive(Debug, Error)]
pub enum ProposerError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
}

impl From<ProposerError> for AppError {
    fn from(value: ProposerError) -> Self {
        AppError::PrepareProposal(value.to_string())
    }
}

pub struct ProposalPreparer {
    params: Shared<Vec<(&'static str, String)>>,
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
            let update_func = || -> Result<(), reqwest::Error> {
                // Copy the params to unlock the mutex.
                let mut params = thread_params.read_access().clone();
                if params.is_empty() {
                    return Ok(());
                }

                // Set the encoding to base64 to match the oracle contract.
                params.push(("encoding", "base64".to_string()));
                // Set the parsed to false since we don't use parsed data.
                params.push(("parsed", "false".to_string()));

                // Retrieve VAAs from pyth node.
                let vaas = reqwest::blocking::Client::builder()
                    .timeout(REQUEST_TIMEOUT)
                    .build()?
                    .get(format!("{PYTH_URL}/v2/updates/price/latest"))
                    .query(&params)
                    .send()?
                    .error_for_status()?
                    .json::<LatestVaaResponse>()?
                    .binary
                    .data;

                info!(len = vaas.len(), "Prepare proposal: fetched latest VAAs");

                // Update the prices.
                thread_latest_vaas.write_with(|mut latest_vaas| {
                    *latest_vaas = vaas;
                });

                Ok(())
            };

            let mut failed_requests: u32 = 0;
            let mut sleep = THREAD_SLEEP;

            loop {
                // Update the VAAs.
                match update_func() {
                    Ok(_) => {
                        failed_requests = 0;
                    },
                    Err(err) => {
                        failed_requests += 1;

                        // Exponentially increases the sleep time on failed requests.
                        sleep = min(
                            MAX_THREAD_SLEEP,
                            THREAD_SLEEP.mul(2u32.pow(failed_requests)),
                        );

                        if let Some(status_code) = err.status() {
                            // The first time we get a 429, we increase
                            // the sleep time to avoid further rate limitation.
                            if status_code == 429 && failed_requests == 1 {
                                sleep = THREAD_SLEEP_ON_FIRST_429;
                            }
                            error!(
                                code = status_code.as_u16(),
                                reason = status_code.canonical_reason().unwrap_or("Unknown"),
                                "Failed to update the latest VAAs"
                            );
                        } else {
                            error!(err = err.to_string(), "Failed to update the latest VAAs");
                        }
                    },
                }

                thread::sleep(sleep);
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
                    Some(("ids[]", id.to_string()))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        // Write the params to the shared memory.
        self.params.write_with(|mut params_ref| {
            *params_ref = params;
        });

        // Retreive the VAAs from the shared memory.
        // Consuming the VAAs to avoid feeding the same prices multiple times.
        let vaas = self.latest_vaas.write_with(|mut prices_lock| {
            let prices = prices_lock.clone();
            *prices_lock = vec![];
            prices
        });

        // Return if there are no VAAs to feed.
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

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod test {
    use {super::LatestVaaResponse, grug::JsonDeExt};

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
