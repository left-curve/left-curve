use {
    crate::pyth_handler::PythHandler,
    dango_types::{config::AppConfig, oracle::ExecuteMsg},
    grug::{Coins, Json, JsonSerExt, Message, NonEmpty, QuerierExt, QuerierWrapper, StdError, Tx},
    prost::bytes::Bytes,
    pyth_client::{PythClient, PythClientCache, PythClientTrait},
    pyth_types::PYTH_URL,
    std::{fmt::Debug, sync::Mutex},
    tracing::error,
};

const GAS_LIMIT: u64 = 50_000_000;

pub struct ProposalPreparer<P> {
    // `Option` to be able to not clone the `PythHandler`.
    pyth_handler: Option<Mutex<PythHandler<P>>>,
}

impl<P> Clone for ProposalPreparer<P> {
    fn clone(&self) -> Self {
        Self { pyth_handler: None }
    }
}

impl ProposalPreparer<PythClient> {
    pub fn new() -> Self {
        let client = PythHandler::new(PYTH_URL);

        Self {
            pyth_handler: Some(Mutex::new(client)),
        }
    }
}

impl Default for ProposalPreparer<PythClient> {
    fn default() -> Self {
        Self::new()
    }
}

impl ProposalPreparer<PythClientCache> {
    pub fn new_with_cache() -> Self {
        let client = PythHandler::new_with_cache(PYTH_URL);

        Self {
            pyth_handler: Some(Mutex::new(client)),
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
        querier: QuerierWrapper,
        mut txs: Vec<Bytes>,
        _max_tx_bytes: usize,
    ) -> Result<Vec<Bytes>, Self::Error> {
        let cfg: AppConfig = querier.query_app_config()?;

        // Should we find a way to start and connect the PythClientPPHandler at startup?
        // How to know which ids should be used?
        let mut pyth_handler = self.pyth_handler.as_ref().unwrap().lock().unwrap();

        // Update the Pyth stream if the PythIds in the oracle have changed.
        if let Err(err) = pyth_handler.update_stream(querier, cfg.addresses.oracle) {
            error!("Failed to update Pyth stream: {:?}", err);
        }

        // Retrieve the VAAs.
        let vaas = pyth_handler.fetch_latest_vaas();

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
