use {
    crate::VmResult,
    clru::CLruCache,
    grug_app::Shared,
    grug_types::Hash,
    std::{collections::HashMap, num::NonZeroUsize},
    wasmer::{Engine, Module},
};

/// An in-memory cache for wasm modules, so that they don't need to be re-built
/// every time the same contract is called.
#[derive(Clone)]
pub struct Cache {
    // Must cache the module together with the engine that was used to built it.
    // There may be runtime errors if calling a wasm function using a different
    // engine from that was used to build the module.
    modules: Shared<CLruCache<Hash, (Module, Engine)>>,
    pub(crate) pinned: Shared<HashMap<Hash, (Module, Engine)>>,
}

impl Cache {
    /// Create an empty cache with the given capacity.
    pub fn new(capacity: NonZeroUsize) -> Self {
        Self {
            modules: Shared::new(CLruCache::new(capacity)),
            pinned: Shared::new(HashMap::default()),
        }
    }

    /// Attempt to get a cached module by hash. If not found, build the module
    /// using the given method, insert the built module into the cache, and
    /// return the module.
    pub fn get_or_build_with<B>(&self, code_hash: &Hash, builder: B) -> VmResult<(Module, Engine)>
    where
        B: FnOnce() -> VmResult<(Module, Engine)>,
    {
        // Pinned: Cache hit - simply clone the module and return
        if let Some(module) = self.pinned.write_access().get(code_hash) {
            return Ok(module.clone());
        }
        // Non pinned: Cache hit - simply clone the module and return
        if let Some(module) = self.modules.write_access().get(code_hash) {
            return Ok(module.clone());
        }

        // Cache miss - build the module using the given builder method; insert
        // both the module and engine to the cache.
        let (module, engine) = builder()?;
        self.modules
            .write_access()
            .put(code_hash.clone(), (module.clone(), engine.clone()));

        Ok((module, engine))
    }

    pub fn try_remove_from_modules(&self, code_hash: &Hash) {
        self.modules.write_access().pop(code_hash);
    }
}
