use {
    crate::{
        QuerierProvider, Shared, SharedGasTracker, Size, StorageProvider, Vm, VmCacheSize, CODES,
        CONTRACT_NAMESPACE,
    },
    clru::{CLruCache, CLruCacheConfig, WeightScale},
    grug_types::{Addr, BlockInfo, Hash, Storage},
    std::{hash::RandomState, marker::PhantomData, num::NonZeroUsize},
};

// Minimum module size.
// Based on `examples/module_size.sh`, and the cosmwasm-plus contracts.
// We use an estimated *minimum* module size in order to compute a number of pre-allocated entries
// that are enough to handle a size-limited cache without requiring re-allocation / resizing.
// This will incurr an extra memory cost for the unused entries, but it's negligible:
// Assuming the cost per entry is 48 bytes, 10000 entries will have an extra cost of just ~500 kB.
// Which is a very small percentage (~0.03%) of our typical cache memory budget (2 GB).
const MINIMUM_MODULE_SIZE: Size = Size::kibi(250);

struct SizeScale<VM> {
    phantom: PhantomData<VM>,
}

impl<VM> Default for SizeScale<VM> {
    fn default() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<VM> WeightScale<Hash, VM::Cache> for SizeScale<VM>
where
    VM: Vm,
{
    #[inline]
    fn weight(&self, key: &Hash, value: &VM::Cache) -> usize {
        std::mem::size_of_val(key) + value.size()
    }
}

pub type SharedCacheVM<VM> = Shared<CacheVM<VM>>;

pub struct CacheVM<VM: Vm> {
    cache: CLruCache<Hash, VM::Cache, RandomState, SizeScale<VM>>,
}

impl<VM> CacheVM<VM>
where
    VM: Vm,
{
    pub fn new(size: Size) -> Self
    where
        VM: Vm,
    {
        let preallocated_entries = size.bytes() / MINIMUM_MODULE_SIZE.bytes();

        Self {
            cache: CLruCache::with_config(
                CLruCacheConfig::new(NonZeroUsize::new(size.bytes()).unwrap())
                    .with_memory(preallocated_entries)
                    .with_scale(SizeScale::default()),
            ),
        }
    }
}

impl<VM: Vm> SharedCacheVM<VM> {
    pub fn build_instance(
        &self,
        storage: Box<dyn Storage>,
        block: BlockInfo,
        address: &Addr,
        code_hash: &Hash,
        gas_tracker: SharedGasTracker,
    ) -> Result<VM, VM::Error> {
        let maybe_cache = self.write_access().cache.get(code_hash).cloned();

        let cache = match maybe_cache {
            Some(cache) => cache,
            None => {
                let code = CODES.load(&storage, code_hash)?;
                let module = VM::build_cache(&code)?;

                // Can we ignore the result??
                let _ = self
                    .write_access()
                    .cache
                    .put_with_weight(code_hash.clone(), module.clone());
                module
            },
        };

        // Create the contract substore and querier
        let substore = StorageProvider::new(storage.clone(), &[CONTRACT_NAMESPACE, address]);
        let querier = QuerierProvider::new(storage, block, gas_tracker.clone(), self.clone());

        VM::build_instance_from_cache(substore, querier, cache, gas_tracker)
    }
}
