use {grug_types::QuerierWrapper, prost::bytes::Bytes};

/// Represents a worker that processes the ABCI++ `PrepareProposal` request.
pub trait ProposalPreparer {
    type Error: ToString;

    /// Process the ABCI++ `PrepareProposal` request.
    ///
    /// The preparer is provided with a querier so that it can do its work based
    /// on the state of the chain.
    fn prepare_proposal(
        &self,
        querier: QuerierWrapper,
        txs: Vec<Bytes>,
        max_tx_bytes: usize,
    ) -> Result<Vec<Bytes>, Self::Error>;
}
