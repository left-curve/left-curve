use {
    crate::{PAIR_PARAMS, PAIR_STATES},
    anyhow::anyhow,
    dango_types::perps::{PairId, PairParam, PairState},
    grug::{Addr, Cache, QuerierWrapper, Storage, StorageQuerier},
    std::{collections::HashMap, rc::Rc},
};

/// An abstraction for querying the parameters of trading pairs with in-memory caching.
pub struct PairQuerier<'a> {
    param_cache: Cache<'a, PairId, PairParam, anyhow::Error, ()>,
    state_cache: Cache<'a, PairId, PairState, anyhow::Error, ()>,
}

impl<'a> PairQuerier<'a> {
    fn new(no_cache_querier: NoCachePairQuerier<'a>) -> Self {
        let no_cache_querier = Rc::new(no_cache_querier);
        let q1 = Rc::clone(&no_cache_querier);
        let q2 = Rc::clone(&no_cache_querier);

        Self {
            param_cache: Cache::new(move |pair_id, _| q1.query_pair_param(pair_id)),
            state_cache: Cache::new(move |pair_id, _| q2.query_pair_state(pair_id)),
        }
    }

    pub fn new_local(storage: &'a dyn Storage) -> Self {
        Self::new(NoCachePairQuerier::new_local(storage))
    }

    pub fn new_remote(address: Addr, querier: QuerierWrapper<'a>) -> Self {
        Self::new(NoCachePairQuerier::new_remote(address, querier))
    }

    pub fn new_mock(
        pair_params: HashMap<PairId, PairParam>,
        pair_states: HashMap<PairId, PairState>,
    ) -> Self {
        Self::new(NoCachePairQuerier::new_mock(pair_params, pair_states))
    }

    pub fn query_pair_param(&mut self, pair_id: &PairId) -> anyhow::Result<&PairParam> {
        self.param_cache.get_or_fetch(pair_id, None)
    }

    pub fn query_pair_state(&mut self, pair_id: &PairId) -> anyhow::Result<&PairState> {
        self.state_cache.get_or_fetch(pair_id, None)
    }
}

/// An abstraction for querying the parameters of trading pairs.
pub enum NoCachePairQuerier<'a> {
    /// Used when perps contract is the current contract.
    Local { storage: &'a dyn Storage },
    /// Used when perps contract is another contract.
    Remote {
        address: Addr,
        querier: QuerierWrapper<'a>,
    },
    /// For testing purpose.
    Mock {
        pair_params: HashMap<PairId, PairParam>,
        pair_states: HashMap<PairId, PairState>,
    },
}

#[rustfmt::skip]
impl<'a> NoCachePairQuerier<'a> {
    pub fn new_local(storage: &'a dyn Storage) -> Self {
        Self::Local { storage }
    }

    pub fn new_remote(address: Addr, querier: QuerierWrapper<'a>) -> Self {
        Self::Remote { address, querier }
    }

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
            Self::Remote { address, querier } => {
                Ok(querier.query_wasm_path(*address, &PAIR_PARAMS.path(pair_id))?)
            },
            Self::Mock { pair_params, .. } => {
                pair_params
                    .get(pair_id)
                    .cloned()
                    .ok_or_else(|| anyhow!("[mock]: pair params not found for pair ID `{pair_id}`"))
            }
        }
    }

    pub fn query_pair_state(&self, pair_id: &PairId) -> anyhow::Result<PairState> {
        match self {
            Self::Local { storage } => {
                Ok(PAIR_STATES.load(*storage, pair_id)?)
            },
            Self::Remote { address, querier } => {
                Ok(querier.query_wasm_path(*address, &PAIR_STATES.path(pair_id))?)
            },
            Self::Mock { pair_states, .. } => {
                pair_states
                    .get(pair_id)
                    .cloned()
                    .ok_or_else(|| anyhow!("[mock]: pair state not found for pair ID `{pair_id}`"))
            }
        }
    }
}
