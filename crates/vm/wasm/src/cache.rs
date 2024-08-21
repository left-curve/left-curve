use {
    crate::VmResult,
    clru::CLruCache,
    grug_app::Shared,
    grug_types::Hash256,
    std::num::NonZeroUsize,
    wasmer::{Engine, Module},
};

pub fn new_cacher(capacity: usize) -> Box<dyn Cacher> {
    if let Some(x) = NonZeroUsize::new(capacity) {
        return Box::new(Cache::new(x));
    }
    Box::new(NoCache::default())
}

pub trait Cacher: Send {
    fn get_or_build_with(
        &self,
        code_hash: Hash256,
        builder: Box<dyn FnOnce() -> VmResult<(Module, Engine)>>,
    ) -> VmResult<(Module, Engine)>;

    fn clone_box(&self) -> Box<dyn Cacher>;
}

impl Clone for Box<dyn Cacher> {
    fn clone(&self) -> Box<dyn Cacher> {
        self.clone_box()
    }
}

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
}

impl Cacher for Cache {
    /// Attempt to get a cached module by hash. If not found, build the module
    /// using the given method, insert the built module into the cache, and
    /// return the module.
    fn get_or_build_with(
        &self,
        code_hash: Hash256,
        builder: Box<dyn FnOnce() -> VmResult<(Module, Engine)>>,
    ) -> VmResult<(Module, Engine)> {
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

    fn clone_box(&self) -> Box<dyn Cacher> {
        Box::new(self.clone())
    }
}

/// It stores nothing, so wasm modules will be re-built
/// every time the same contract is called.
#[derive(Clone)]
pub struct NoCache {}

impl NoCache {
    /// Create a NoCache
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for NoCache {
    fn default() -> Self {
        Self::new()
    }
}

impl Cacher for NoCache {
    fn get_or_build_with(
        &self,
        _code_hash: Hash256,
        builder: Box<dyn FnOnce() -> VmResult<(Module, Engine)>>,
    ) -> VmResult<(Module, Engine)> {
        let (module, engine) = builder()?;
        Ok((module, engine))
    }

    fn clone_box(&self) -> Box<dyn Cacher> {
        Box::new(self.clone())
    }
}
