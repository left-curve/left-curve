use {
    crate::GasTracker,
    grug_types::{BlockInfo, Storage, Undefined},
    std::mem,
};

/// The context under which app functions are executed.
///
/// Includes the virtual machine used, and key-value storage being operated on,
/// gas tracker, block, and so on.
#[derive(Clone)]
pub struct AppCtx<VM = Undefined, S = Box<dyn Storage>> {
    pub vm: VM,
    pub storage: S,
    pub gas_tracker: GasTracker,
    pub chain_id: String,
    pub block: BlockInfo,
}

impl<VM, S> AppCtx<VM, S> {
    /// Create a new context.
    pub fn new<C>(
        vm: VM,
        storage: S,
        gas_tracker: GasTracker,
        chain_id: C,
        block: BlockInfo,
    ) -> Self
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

    /// Replace the gas tracker with a new one; return the old one.
    pub fn replace_gas_tracker(&mut self, gas_tracker: GasTracker) -> GasTracker {
        mem::replace(&mut self.gas_tracker, gas_tracker)
    }

    /// Cast the context to a variant where the VM is undefined.
    ///
    /// Used for methods that don't require a VM, such as updating chain config
    /// or uploading a code.
    pub fn downcast(self) -> AppCtx<Undefined, S> {
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
    /// Clone the context, at the same time replacing the storage with a new one.
    pub fn clone_with_storage<S1>(&self, storage: S1) -> AppCtx<VM, S1> {
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
    /// Clone the context, at the same time put the storage in a `Box`.
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
