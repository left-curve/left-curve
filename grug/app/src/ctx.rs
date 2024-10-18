use {
    crate::{Buffer, GasTracker, Shared, Vm},
    grug_types::{BlockInfo, Storage, Undefined},
};

#[derive(Clone)]
pub struct AppCtx<VM = Undefined<()>, S = Box<dyn Storage>> {
    pub vm: VM,
    pub storage: S,
    pub block: BlockInfo,
    pub gas_tracker: GasTracker,
}

impl<VM> AppCtx<VM>
where
    VM: Vm,
{
    pub fn new<S>(vm: VM, storage: S, gas_tracker: GasTracker, block: BlockInfo) -> AppCtx<VM, S> {
        AppCtx {
            storage,
            block,
            gas_tracker,
            vm,
        }
    }

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

    pub fn downcast(&self) -> AppCtx<Undefined<()>> {
        AppCtx {
            storage: self.storage.clone(),
            block: self.block.clone(),
            gas_tracker: self.gas_tracker.clone(),
            vm: Undefined::default(),
        }
    }
}

impl<VM, S> AppCtx<VM, S>
where
    VM: Vm + Clone,
{
    pub fn clone_with_storage<S1>(&self, storage: S1) -> AppCtx<VM, S1> {
        AppCtx {
            storage,
            block: self.block.clone(),
            gas_tracker: self.gas_tracker.clone(),
            vm: self.vm.clone(),
        }
    }
}

impl<VM> AppCtx<VM, Box<dyn Storage>>
where
    VM: Vm + Clone,
{
    pub fn with_buffer_storage<S1>(
        &self,
        storage: Shared<Buffer<S1>>,
    ) -> AppCtx<VM, Shared<Buffer<S1>>> {
        AppCtx {
            storage,
            block: self.block.clone(),
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
    pub fn box_me(&self) -> AppCtx<VM, Box<dyn Storage>> {
        AppCtx {
            storage: Box::new(self.storage.clone()),
            block: self.block.clone(),
            gas_tracker: self.gas_tracker.clone(),
            vm: self.vm.clone(),
        }
    }
}
