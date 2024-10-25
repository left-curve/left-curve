use {
    crate::{Buffer, GasTracker, Shared, Vm},
    grug_types::{BlockInfo, Storage, Undefined},
};

/// Wrapper `ctx` struct that holds the `VM`, `storage`, `block`, and `gas_tracker`
#[derive(Clone)]
pub struct AppCtx<VM = Undefined, S = Box<dyn Storage>> {
    pub block: BlockInfo,
    pub chain_id: String,
    pub gas_tracker: GasTracker,
    pub storage: S,
    vm: VM,
}

impl AppCtx {
    pub fn new<VM, S, C>(
        block: BlockInfo,
        chain_id: C,
        gas_tracker: GasTracker,
        storage: S,
        vm: VM,
    ) -> AppCtx<VM, S>
    where
        C: Into<String>,
    {
        AppCtx {
            block,
            chain_id: chain_id.into(),
            gas_tracker,
            storage,
            vm,
        }
    }
}

impl<VM> AppCtx<VM>
where
    VM: Vm,
{
    pub fn vm(&self) -> &VM {
        &self.vm
    }

    /// Downcast the `AppCtx<VM>` to `AppCtx<Undefined>`
    pub fn downcast(&self) -> AppCtx<Undefined> {
        AppCtx {
            block: self.block,
            chain_id: self.chain_id.clone(),
            gas_tracker: self.gas_tracker.clone(),
            storage: self.storage.clone(),
            vm: Undefined::default(),
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
            block: self.block,
            chain_id: self.chain_id.clone(),
            gas_tracker: self.gas_tracker.clone(),
            storage,
            vm: self.vm.clone(),
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
            block: self.block,
            chain_id: self.chain_id.clone(),
            gas_tracker: self.gas_tracker.clone(),
            storage,
            vm: self.vm.clone(),
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
            block: self.block,
            chain_id: self.chain_id.clone(),
            gas_tracker: self.gas_tracker.clone(),
            storage: Box::new(self.storage.clone()),
            vm: self.vm.clone(),
        }
    }
}
