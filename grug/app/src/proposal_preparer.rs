#[cfg(feature = "tracing")]
use tracing::info;
use {
    crate::{AppError, ProposalPreparer},
    grug_types::{Querier, QuerierWrapper, Query, QueryResponse, StdResult},
    prost::bytes::Bytes,
    std::{
        convert::Infallible,
        fmt::{self, Display},
    },
};

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
        _querier: QuerierWrapper,
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

/// A placeholder error type that is never constructed.
/// Used in conjunction with [`NaiveProposalPreparer`](crate::NaiveProposalPreparer).
#[derive(Debug, Clone, Copy)]
pub struct NaiveError(Infallible);

impl Display for NaiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<NaiveError> for AppError {
    fn from(err: NaiveError) -> Self {
        AppError::PrepareProposal(err.to_string())
    }
}

/// A querier that doesn't actually perform any query.
/// Used in conjunction with [`NaiveProposalPreparer`](crate::NaiveProposalPreparer).
#[derive(Debug, Clone, Copy)]
pub struct NaiveQuerier;

impl Querier for NaiveQuerier {
    fn query_chain(&self, _req: Query) -> StdResult<QueryResponse> {
        unreachable!("attempting to query a no-op querier");
    }
}
