use {
    crate::PAIRS,
    anyhow::anyhow,
    dango_types::dex::PairParams,
    grug::{Addr, Cache, Denom, QuerierWrapper, StorageQuerier},
};

pub struct PairQuerier<'a> {
    cache: Cache<'a, (Denom, Denom), PairParams, anyhow::Error>,
}

impl<'a> PairQuerier<'a> {
    pub fn new(contract: Addr, querier: QuerierWrapper<'a>) -> Self {
        Self {
            cache: Cache::new(move |(base_denom, quote_denom), _| {
                querier
                    .may_query_wasm_path(contract, &PAIRS.path((base_denom, quote_denom)))?
                    .ok_or_else(|| {
                        anyhow!(
                            "pair not found with base `{}` and quote `{}`",
                            base_denom,
                            quote_denom
                        )
                    })
            }),
        }
    }

    pub fn query_pair(
        &mut self,
        base_denom: Denom,
        quote_denom: Denom,
    ) -> anyhow::Result<&PairParams> {
        self.cache.get_or_fetch(&(base_denom, quote_denom), None)
    }
}
