#[cfg(test)]
use std::collections::HashMap;
use {
    crate::state::{PAIR_PARAMS, PAIR_STATES},
    dango_order_book::PairId,
    dango_types::perps::{PairParam, PairState},
    grug::Storage,
};

/// An abstraction for querying perps contract state.
pub enum NoCachePerpQuerier<'a> {
    /// Used when perps contract is the current contract.
    Local { storage: &'a dyn Storage },
    /// For testing purpose.
    #[cfg(test)]
    Mock {
        pair_params: HashMap<PairId, PairParam>,
        pair_states: HashMap<PairId, PairState>,
    },
}

#[rustfmt::skip]
impl<'a> NoCachePerpQuerier<'a> {
    pub fn new_local(storage: &'a dyn Storage) -> Self {
        Self::Local { storage }
    }

    #[cfg(test)]
    pub fn new_mock(
        pair_params: HashMap<PairId, PairParam>,
        pair_states: HashMap<PairId, PairState>,
    ) -> Self {
        Self::Mock { pair_params, pair_states }
    }

    pub fn query_pair_param(&self, pair_id: &PairId) -> anyhow::Result<PairParam> {
        match self {
            Self::Local { storage } => {
                Ok(PAIR_PARAMS.load(*storage, pair_id)?)
            },
            #[cfg(test)]
            Self::Mock { pair_params, .. } => {
                pair_params
                    .get(pair_id)
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("[mock]: pair params not found for pair ID `{pair_id}`"))
            },
        }
    }

    pub fn query_pair_state(&self, pair_id: &PairId) -> anyhow::Result<PairState> {
        match self {
            Self::Local { storage } => {
                Ok(PAIR_STATES.load(*storage, pair_id)?)
            },
            #[cfg(test)]
            Self::Mock { pair_states, .. } => {
                pair_states
                    .get(pair_id)
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("[mock]: pair state not found for pair ID `{pair_id}`"))
            },
        }
    }
}
