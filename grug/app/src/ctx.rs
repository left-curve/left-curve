use {
    crate::{Buffer, GasTracker, Shared, Vm},
    grug_types::{BlockInfo, Storage, Undefined},
};

/// Wrapper `ctx` struct that holds the `VM`, `storage`, `block`, and `gas_tracker`
#[derive(Clone)]
pub struct AppCtx<VM = Undefined<()>, S = Box<dyn Storage>> {
    vm: VM,
    pub storage: S,
    pub block: BlockInfo,
    pub gas_tracker: GasTracker,
}

impl<VM> AppCtx<VM>
where
    VM: Vm,
{
    /// Constructor method where `S` can be any type that implements `Storage`
    ///
    /// This is needed to have `Shared<Buffer>` as storage in order to dismount or commit the buffer
    pub fn new<S>(vm: VM, storage: S, gas_tracker: GasTracker, block: BlockInfo) -> AppCtx<VM, S> {
        AppCtx {
            storage,
            block,
            gas_tracker,
            vm,
        }
    }

    /// Constructor method where `S` is a `Box<dyn Storage>`
    pub fn new_boxed(
        vm: VM,
        storage: Box<dyn Storage>,
        gas_tracker: GasTracker,
        block: BlockInfo,
    ) -> AppCtx<VM, Box<dyn Storage>> {
        AppCtx {
            storage,
            block,
            gas_tracker,
            vm,
        }
    }

    pub fn vm(&self) -> &VM {
        &self.vm
    }

    /// Downcast the `AppCtx<VM>` to `AppCtx<Undefined<()>>`
    pub fn downcast(&self) -> AppCtx<Undefined<()>> {
        AppCtx {
            storage: self.storage.clone(),
            block: self.block,
            gas_tracker: self.gas_tracker.clone(),
            vm: Undefined::default(),
        }
    }
}

impl<VM, S> AppCtx<VM, S>
where
    VM: Vm + Clone,
{
    /// Clone the `AppCtx` replacing the storage with a generic `Storage`
    pub fn clone_with_storage<S1>(&self, storage: S1) -> AppCtx<VM, S1>
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

impl<VM> AppCtx<VM, Box<dyn Storage>>
where
    VM: Vm + Clone,
{
    /// Clone the `AppCtx` that is using a `Box<dyn Storage>` replacing it with a new `Shared<Buffer<S>>`
    pub fn clone_with_buffer_storage<S1>(
        &self,
        storage: Shared<Buffer<S1>>,
    ) -> AppCtx<VM, Shared<Buffer<S1>>> {
        AppCtx {
            storage,
            block: self.block,
            gas_tracker: self.gas_tracker.clone(),
            vm: self.vm.clone(),
        }
    }
}

impl<VM, S> AppCtx<VM, S>
where
    VM: Vm + Clone,
    S: Storage + Clone + 'static,
{
    /// Clone the `AppCtx` boxing the inner `storage`
    pub fn clone_boxing_storage(&self) -> AppCtx<VM, Box<dyn Storage>> {
        AppCtx {
            storage: Box::new(self.storage.clone()),
            block: self.block,
            gas_tracker: self.gas_tracker.clone(),
            vm: self.vm.clone(),
        }
    }
}
