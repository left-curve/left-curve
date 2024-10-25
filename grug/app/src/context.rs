use {
    crate::{Buffer, GasTracker, Shared, Vm},
    grug_types::{BlockInfo, Storage, Undefined},
};

/// Wrapper `ctx` struct that holds the `VM`, `storage`, `block`, and `gas_tracker`
#[derive(Clone)]
pub struct AppCtx<VM = Undefined, S = Box<dyn Storage>> {
    pub vm: VM,
    pub storage: S,
    pub gas_tracker: GasTracker,
    pub chain_id: String,
    pub block: BlockInfo,
}

impl AppCtx {
    pub fn new<VM, S, C>(
        vm: VM,
        storage: S,
        gas_tracker: GasTracker,
        chain_id: C,
        block: BlockInfo,
    ) -> AppCtx<VM, S>
    where
        C: Into<String>,
    {
        AppCtx {
            vm,
            storage,
            gas_tracker,
            chain_id: chain_id.into(),
            block,
        }
    }
}

impl<VM> AppCtx<VM>
where
    VM: Vm,
{
    /// Downcast the `AppCtx<VM>` to `AppCtx<Undefined>`
    pub fn downcast(self) -> AppCtx<Undefined> {
        AppCtx {
            vm: Undefined::default(),
            storage: self.storage,
            gas_tracker: self.gas_tracker,
            chain_id: self.chain_id,
            block: self.block,
        }
    }
}

impl<VM, S> AppCtx<VM, S>
where
    VM: Clone,
{
    /// Clone the `AppCtx` replacing the storage with a generic `Storage`
    pub fn clone_with_storage<S1>(&self, storage: S1) -> AppCtx<VM, S1>
    where
        S1: Storage,
    {
        AppCtx {
            vm: self.vm.clone(),
            storage,
            gas_tracker: self.gas_tracker.clone(),
            chain_id: self.chain_id.clone(),
            block: self.block,
        }
    }
}

impl<VM> AppCtx<VM, Box<dyn Storage>>
where
    VM: Clone,
{
    /// Clone the `AppCtx` that is using a `Box<dyn Storage>` replacing it with a new `Shared<Buffer<S>>`
    pub fn clone_with_buffer_storage<S1>(
        &self,
        storage: Shared<Buffer<S1>>,
    ) -> AppCtx<VM, Shared<Buffer<S1>>> {
        AppCtx {
            vm: self.vm.clone(),
            storage,
            gas_tracker: self.gas_tracker.clone(),
            chain_id: self.chain_id.clone(),
            block: self.block,
        }
    }
}

impl<VM, S> AppCtx<VM, S>
where
    VM: Clone,
    S: Storage + Clone + 'static,
{
    /// Clone the `AppCtx` boxing the inner `storage`
    pub fn clone_boxing_storage(&self) -> AppCtx<VM, Box<dyn Storage>> {
        AppCtx {
            vm: self.vm.clone(),
            storage: Box::new(self.storage.clone()),
            gas_tracker: self.gas_tracker.clone(),
            chain_id: self.chain_id.clone(),
            block: self.block,
        }
    }
}
