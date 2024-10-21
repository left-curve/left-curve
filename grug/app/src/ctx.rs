use {
    crate::{Buffer, GLimitLess, GLimited, GUnbound, GasTracker, Shared},
    grug_types::{BlockInfo, Storage, Undefined},
};

/// Wrapper `ctx` struct that holds the `VM`, `storage`, `block`, and `gas_tracker`
pub struct AppCtx<VM = Undefined<()>, G = GUnbound, S = Box<dyn Storage>> {
    vm: VM,
    pub storage: S,
    pub block: BlockInfo,
    pub gas_tracker: GasTracker<G>,
}

impl<VM, G, S> Clone for AppCtx<VM, G, S>
where
    VM: Clone,
    S: Clone,
{
    fn clone(&self) -> Self {
        Self {
            vm: self.vm.clone(),
            storage: self.storage.clone(),
            block: self.block.clone(),
            gas_tracker: self.gas_tracker.clone(),
        }
    }
}

impl AppCtx {
    /// Constructor method where `S` can be any type that implements `Storage`
    ///
    /// This is needed to have `Shared<Buffer>` as storage in order to dismount or commit the buffer
    pub fn new<VM, S, G>(
        vm: VM,
        storage: S,
        gas_tracker: GasTracker<G>,
        block: BlockInfo,
    ) -> AppCtx<VM, G, S> {
        AppCtx {
            storage,
            block,
            gas_tracker,
            vm,
        }
    }

    /// Constructor method where `S` is a `Box<dyn Storage>`
    pub fn new_boxed<VM, G>(
        vm: VM,
        storage: Box<dyn Storage>,
        gas_tracker: GasTracker<G>,
        block: BlockInfo,
    ) -> AppCtx<VM, G, Box<dyn Storage>> {
        AppCtx {
            storage,
            block,
            gas_tracker,
            vm,
        }
    }
}

impl<VM, G, S> AppCtx<VM, G, S>
where
    S: Clone,
{
    /// Downcast the `AppCtx<VM>` to `AppCtx<Undefined<()>>`
    pub fn downcast(&self) -> AppCtx<Undefined<()>, G, S> {
        AppCtx {
            storage: self.storage.clone(),
            block: self.block,
            gas_tracker: self.gas_tracker.clone(),
            vm: Undefined::default(),
        }
    }
}

impl<VM, G, S> AppCtx<VM, G, S> {
    pub fn vm(&self) -> &VM {
        &self.vm
    }
}

impl<VM, G, S> AppCtx<VM, G, S>
where
    VM: Clone,
{
    /// Clone the `AppCtx` replacing the storage with a generic `Storage`
    pub fn clone_with_storage<S1>(&self, storage: S1) -> AppCtx<VM, G, S1>
    where
        S1: Storage,
    {
        AppCtx {
            storage,
            block: self.block,
            gas_tracker: self.gas_tracker.clone(),
            vm: self.vm.clone(),
        }
    }
}

impl<VM, G> AppCtx<VM, G, Box<dyn Storage>>
where
    VM: Clone,
{
    /// Clone the `AppCtx` that is using a `Box<dyn Storage>` replacing it with a new `Shared<Buffer<S>>`
    pub fn clone_with_buffer_storage<S1>(
        &self,
        storage: Shared<Buffer<S1>>,
    ) -> AppCtx<VM, G, Shared<Buffer<S1>>> {
        AppCtx {
            storage,
            block: self.block,
            gas_tracker: self.gas_tracker.clone(),
            vm: self.vm.clone(),
        }
    }
}

impl<VM, G, S> AppCtx<VM, G, S>
where
    VM: Clone,
    S: Storage + Clone + 'static,
{
    /// Clone the `AppCtx` boxing the inner `storage`
    pub fn clone_boxing_storage(&self) -> AppCtx<VM, G, Box<dyn Storage>> {
        AppCtx {
            storage: Box::new(self.storage.clone()),
            block: self.block,
            gas_tracker: self.gas_tracker.clone(),
            vm: self.vm.clone(),
        }
    }
}

impl<VM, G, S> AppCtx<VM, G, S> {
    pub fn clone_with_gas_tracker<G1>(self, gas_tracker: GasTracker<G1>) -> AppCtx<VM, G1, S> {
        AppCtx {
            storage: self.storage,
            block: self.block,
            gas_tracker,
            vm: self.vm,
        }
    }
}

impl<VM, S> AppCtx<VM, GLimitLess, S> {
    pub fn unbound(self) -> AppCtx<VM, GUnbound, S> {
        AppCtx {
            storage: self.storage,
            block: self.block,
            gas_tracker: self.gas_tracker.unbound(),
            vm: self.vm,
        }
    }
}

impl<VM, S> AppCtx<VM, GLimited, S> {
    pub fn unbound(self) -> AppCtx<VM, GUnbound, S> {
        AppCtx {
            storage: self.storage,
            block: self.block,
            gas_tracker: self.gas_tracker.unbound(),
            vm: self.vm,
        }
    }
}
