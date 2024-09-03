use {
    crate::VmResult,
    clru::CLruCache,
    grug_app::Shared,
    grug_types::Hash256,
    std::num::NonZeroUsize,
    wasmer::{Engine, Module},
};

/// An in-memory cache for wasm modules, so that they don't need to be re-built
/// every time the same contract is called.
#[derive(Clone)]
pub struct Cache {
    // Must cache the module together with the engine that was used to built it.
    // There may be runtime errors if calling a wasm function using a different
    // engine from that was used to build the module.
    inner: Shared<CLruCache<Hash256, (Module, Engine)>>,
}

impl Cache {
    /// Create an empty cache with the given capacity.
    pub fn new(capacity: NonZeroUsize) -> Self {
        Self {
            inner: Shared::new(CLruCache::new(capacity)),
        }
    }

    /// Attempt to get a cached module by hash. If not found, build the module
    /// using the given method, insert the built module into the cache, and
    /// return the module.
    pub fn get_or_build_with<B>(&self, code_hash: Hash256, builder: B) -> VmResult<(Module, Engine)>
    where
        B: FnOnce() -> VmResult<(Module, Engine)>,
    {
        // Cache hit - simply clone the module and return
        if let Some(module) = self.inner.write_access().get(&code_hash) {
            return Ok(module.clone());
        }

        // Cache miss - build the module using the given builder method; insert
        // both the module and engine to the cache.
        let (module, engine) = builder()?;
        self.inner
            .write_access()
            .put(code_hash, (module.clone(), engine.clone()));

        Ok((module, engine))
    }
}
#[cfg(test)]
mod tests {
    use {
        crate::{Cache, VmResult},
        grug_types::HashExt,
        std::num::NonZeroUsize,
        wasmer::{Engine, Module, Singlepass},
    };

    const CONTRACT: &[u8] = br#"(module)"#;

    fn builder() -> VmResult<(Module, Engine)> {
        let engine = Engine::from(Singlepass::new());
        let module = Module::new(&engine, CONTRACT)?;
        Ok((module, engine))
    }

    #[test]
    fn capacity_overflow() {
        let cache = Cache::new(NonZeroUsize::new(1).unwrap());
        let hash = CONTRACT.hash256();
        cache.get_or_build_with(hash, builder).unwrap();
        let hash2 = b"jake".hash256();
        cache.get_or_build_with(hash2, builder).unwrap();
        assert_eq!(cache.inner.read_access().len(), 1);
    }

    #[test]
    fn get_cached() {
        let cache = Cache::new(NonZeroUsize::new(2).unwrap());
        let hash = CONTRACT.hash256();
        cache.get_or_build_with(hash, builder).unwrap();
        cache.get_or_build_with(hash, builder).unwrap();
        assert_eq!(cache.inner.read_access().len(), 1);
    }
}
