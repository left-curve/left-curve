use {crate::ProposalPreparer, prost::bytes::Bytes, std::convert::Infallible, tracing::info};

/// A proposal preparer that implements a naive strategy of simply removing
/// transactions from end of the list until the list is within the size limit.
///
/// Forked from [tendermint-rs](https://github.com/informalsystems/tendermint-rs/blob/v0.40.0/abci/src/application.rs#L100-L124),
/// which is released under [Apache-2.0 license](https://github.com/informalsystems/tendermint-rs/blob/v0.40.0/LICENSE).
#[derive(Debug, Clone, Copy)]
pub struct NaiveProposalPreparer;

impl ProposalPreparer for NaiveProposalPreparer {
    type Error = Infallible;

    fn prepare_proposal(
        &self,
        mut txs: Vec<Bytes>,
        max_tx_bytes: usize,
    ) -> Result<Vec<Bytes>, Self::Error> {
        let mut total_tx_bytes: usize = txs
            .iter()
            .map(|tx| tx.len())
            .fold(0, |acc, len| acc.saturating_add(len));

        while total_tx_bytes > max_tx_bytes {
            if let Some(tx) = txs.pop() {
                total_tx_bytes = total_tx_bytes.saturating_sub(tx.len());
            } else {
                break;
            }
        }

        #[cfg(feature = "tracing")]
        info!(num_txs = txs.len(), "Prepared proposal");

        Ok(txs)
    }
}
