use {
    crate::{Iterator, VmError, VmResult, WasmVm},
    grug_app::{GasTracker, QuerierProvider, StorageProvider},
    grug_types::{Record, StdError},
    std::{collections::HashMap, ptr::NonNull},
    wasmer::{AsStoreMut, AsStoreRef, Instance, Memory, MemoryView, Value},
    wasmer_middlewares::metering::{get_remaining_points, set_remaining_points, MeteringPoints},
};

/// Necessary stuff for performing Wasm import functions.
pub struct Environment {
    pub storage: StorageProvider,
    pub storage_readonly: bool,
    pub querier: QuerierProvider<WasmVm>,
    pub query_depth: usize,
    pub gas_tracker: GasTracker,
    /// The amount of gas points remaining in the `Metering` middleware the last
    /// time we updated the `gas_tracker`.
    ///
    /// Comparing this number with the current amount of remaining gas in the
    /// meter, we can determine how much gas was consumed since the last update.
    gas_checkpoint: u64,
    /// Active iterators, indexed by IDs.
    iterators: HashMap<i32, Iterator>,
    /// If a new iterator is to be added, it's ID will be this. Incremented each
    /// time a new iterator is added.
    next_iterator_id: i32,
    /// Memory of the Wasmer instance. Necessary for reading data from or
    /// writing data to the memory.
    ///
    /// Optional because during the flow of creating the Wasmer instance, the
    /// `Environment` needs to be created before the instance, which the memory
    /// is a part of.
    ///
    /// Therefore, we set this to `None` first, then after the instance is
    /// created, use the `set_wasmer_memory` method to set it.
    wasmer_memory: Option<Memory>,
    /// A non-owning link to the Wasmer instance. Necessary for doing function
    /// calls (see `Environment::call_function`).
    ///
    /// Optional for the same reason as `wasmer_memory`.
    wasmer_instance: Option<NonNull<Instance>>,
}

// The Wasmer instance isn't `Send`. We manually mark it as is.
// cosmwasm-vm does the same:
// https://github.com/CosmWasm/cosmwasm/blob/v2.0.3/packages/vm/src/environment.rs#L120-L122
// TODO: need to think about whether this is safe
unsafe impl Send for Environment {}

impl Environment {
    pub fn new(
        storage: StorageProvider,
        storage_readonly: bool,
        querier: QuerierProvider<WasmVm>,
        query_depth: usize,
        gas_tracker: GasTracker,
        gas_checkpoint: u64,
    ) -> Self {
        Self {
            storage,
            storage_readonly,
            querier,
            query_depth,
            gas_tracker,
            gas_checkpoint,
            iterators: HashMap::new(),
            next_iterator_id: 0,
            // Wasmer memory and instance are set to `None` because at this
            // point, the Wasmer instance hasn't been created yet.
            wasmer_memory: None,
            wasmer_instance: None,
        }
    }

    /// Add a new iterator to the `Environment`, increment the next iterator ID.
    ///
    /// Return the ID of the iterator that was just added.
    pub fn add_iterator(&mut self, iterator: Iterator) -> i32 {
        let iterator_id = self.next_iterator_id;
        self.iterators.insert(iterator_id, iterator);
        self.next_iterator_id += 1;

        iterator_id
    }

    /// Get the next record in the iterator specified by the ID.
    ///
    /// Error if the iterator is not found.
    /// `None` if the iterator is found but has reached its end.
    pub fn advance_iterator(&mut self, iterator_id: i32) -> VmResult<Option<Record>> {
        self.iterators
            .get_mut(&iterator_id)
            .ok_or(VmError::IteratorNotFound { iterator_id })
            .map(|iter| iter.next(&self.storage))
    }

    /// Delete all existing iterators.
    ///
    /// This is called when an import that mutates the storage (namely,
    /// `db_write`, `db_remove`, and `db_remove_range`) is called, because the
    /// mutations may change the iteration.
    ///
    /// Note that we don't reset the `next_iterator_id` though.
    pub fn clear_iterators(&mut self) {
        self.iterators.clear();
    }

    pub fn set_wasmer_memory(&mut self, instance: &Instance) -> VmResult<()> {
        if self.wasmer_memory.is_some() {
            return Err(VmError::WasmerMemoryAlreadySet);
        }

        let memory = instance.exports.get_memory("memory")?;
        self.wasmer_memory = Some(memory.clone());

        Ok(())
    }

    pub fn set_wasmer_instance(&mut self, instance: &Instance) -> VmResult<()> {
        if self.wasmer_instance.is_some() {
            return Err(VmError::WasmerInstanceAlreadySet);
        }

        self.wasmer_instance = Some(NonNull::from(instance));

        Ok(())
    }

    pub fn get_wasmer_memory<'a, S>(&self, store: &'a S) -> VmResult<MemoryView<'a>>
    where
        S: AsStoreRef,
    {
        self.wasmer_memory
            .as_ref()
            .ok_or(VmError::WasmerMemoryNotSet)
            .map(|mem| mem.view(store))
    }

    pub fn get_wasmer_instance(&self) -> VmResult<&Instance> {
        let instance_ptr = self.wasmer_instance.ok_or(VmError::WasmerInstanceNotSet)?;
        unsafe { Ok(instance_ptr.as_ref()) }
    }

    pub fn call_function1<S>(
        &mut self,
        store: &mut S,
        name: &'static str,
        args: &[Value],
    ) -> VmResult<Value>
    where
        S: AsStoreMut,
    {
        let ret = self.call_function(store, name, args)?;
        if ret.len() != 1 {
            return Err(VmError::ReturnCount {
                name: name.into(),
                expect: 1,
                actual: ret.len(),
            });
        }
        Ok(ret[0].clone())
    }

    pub fn call_function0<S>(
        &mut self,
        store: &mut S,
        name: &'static str,
        args: &[Value],
    ) -> VmResult<()>
    where
        S: AsStoreMut,
    {
        let ret = self.call_function(store, name, args)?;
        if ret.len() != 0 {
            return Err(VmError::ReturnCount {
                name: name.into(),
                expect: 0,
                actual: ret.len(),
            });
        }
        Ok(())
    }

    fn call_function<S>(
        &mut self,
        store: &mut S,
        name: &'static str,
        args: &[Value],
    ) -> VmResult<Box<[Value]>>
    where
        S: AsStoreMut,
    {
        let instance = self.get_wasmer_instance()?;
        let func = instance.exports.get_function(name)?;

        // Make the function call. Then, regardless of whether the call succeeds
        // or fails, check the remaining gas points.
        match (
            func.call(store, args),
            get_remaining_points(store, instance),
        ) {
            // The call has succeeded, or has failed but for a reason other than
            // running out of gas. In such cases, we update the gas tracker, and
            // return the result as-is.
            (result, MeteringPoints::Remaining(remaining)) => {
                let consumed = self.gas_checkpoint - remaining;
                self.gas_tracker.consume(consumed, name)?;
                self.gas_checkpoint = remaining;

                Ok(result?)
            },
            // The call has failed because of running out of gas.
            //
            // Firstly, we update the gas tracker, because this call may be
            // triggered by a submessage with "reply on error", so the transaction
            // handling may not be aborted yet.
            //
            // Secondly, we return a "gas depletion" error instead. Wasmer's
            // default error would be "VM error: unreachable" in this case,
            // which isn't very helpful.
            (Err(_), MeteringPoints::Exhausted) => {
                // Note that if an _unlimited_ gas call goes out of gas (meaning
                // all `u64::MAX` gas units have been depleted) this would
                // overflow. However this should never happen in practice (the
                // call would run an exceedingly long time to start with).
                self.gas_tracker.consume(self.gas_checkpoint, name)?;
                self.gas_checkpoint = 0;

                Err(StdError::OutOfGas {
                    limit: self.gas_tracker.limit().unwrap_or(u64::MAX),
                    used: self.gas_tracker.used(),
                    comment: name,
                }
                .into())
            },
            // The call succeeded, but gas depleted: impossible senario.
            (Ok(_), MeteringPoints::Exhausted) => {
                unreachable!("No way! Gas is depleted but call is successful.");
            },
        }
    }

    /// Record gas consumed by host functions.
    pub fn consume_external_gas<S>(
        &mut self,
        store: &mut S,
        external: u64,
        comment: &'static str,
    ) -> VmResult<()>
    where
        S: AsStoreMut,
    {
        let instance = self.get_wasmer_instance()?;
        match get_remaining_points(store, instance) {
            MeteringPoints::Remaining(remaining) => {
                // gas_checkpoint can't be less than remaining
                // compute consumed equals to the gas consumed since the last update + external gas
                let consumed = self.gas_checkpoint - remaining + external;
                self.gas_tracker.consume(consumed, comment)?;

                // If there is a limit on gas_tracker, update the remaining points in the store
                if let Some(remaining) = self.gas_tracker.remaining() {
                    set_remaining_points(store, instance, remaining);
                    self.gas_checkpoint = remaining;
                }
                Ok(())
            },
            // The contract made a host function call, but gas depleted; impossible.
            MeteringPoints::Exhausted => {
                unreachable!("No way! Gas is depleted but contract made a host function call.");
            },
        }
    }
}
