use {
    crate::pyth_handler::{PythHandler, QueryPythId},
    grug::{Lengthy, NonEmpty, QuerierWrapper, StdError},
    prost::bytes::Bytes,
    pyth_client::{PythClient, PythClientCache, PythClientTrait},
    pyth_types::constants::LAZER_ENDPOINTS_TEST,
    reqwest::IntoUrl,
    std::fmt::Debug,
};

pub struct ProposalPreparer<P>
where
    P: PythClientTrait + QueryPythId,
{
    pyth: PythHandler<P>,
}

impl<P> Clone for ProposalPreparer<P>
where
    P: PythClientTrait + QueryPythId,
{
    fn clone(&self) -> Self {
        Self {
            pyth: self.pyth.clone(),
        }
    }
}

impl ProposalPreparer<PythClient> {
    pub fn new<V, U, T>(endpoints: V, access_token: T) -> Self
    where
        V: IntoIterator<Item = U> + Lengthy,
        U: IntoUrl,
        T: ToString,
    {
        Self {
            pyth: PythHandler::new(endpoints, access_token),
        }
    }
}

impl ProposalPreparer<PythClientCache> {
    pub fn new_with_cache() -> Self {
        // `LAZER_ENDPOINTS_TEST` is a static non-empty array.
        Self {
            pyth: PythHandler::new_with_cache(
                NonEmpty::new_unchecked(LAZER_ENDPOINTS_TEST),
                "lazer_token",
            ),
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
        txs: Vec<Bytes>,
        max_tx_bytes: usize,
    ) -> Result<Vec<Bytes>, Self::Error> {
        self.pyth.prepare_proposal(querier, txs, max_tx_bytes)
    }
}
