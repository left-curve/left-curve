use {
    crate::{Iterator, VmError, VmResult},
    grug_app::{GasTracker, QuerierProvider, StorageProvider},
    grug_types::{Record, StdError},
    std::{collections::HashMap, ptr::NonNull},
    wasmer::{AsStoreMut, AsStoreRef, Instance, Memory, MemoryView, Value},
    wasmer_middlewares::metering::{MeteringPoints, get_remaining_points, set_remaining_points},
};

/// Necessary stuff for performing Wasm import functions.
pub struct Environment {
    pub storage: StorageProvider,
    pub state_mutable: bool,
    pub querier: Box<dyn QuerierProvider>,
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
        state_mutable: bool,
        querier: Box<dyn QuerierProvider>,
        query_depth: usize,
        gas_tracker: GasTracker,
        gas_checkpoint: u64,
    ) -> Self {
        Self {
            storage,
            state_mutable,
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

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod test {
    use {
        crate::{Environment, GAS_PER_OPERATION, Iterator, VmError, VmResult, WasmVm},
        grug_app::{GasTracker, QuerierProviderImpl, Shared, StorageProvider},
        grug_types::{BlockInfo, Hash256, MockStorage, Order, StdError, Storage, Timestamp},
        std::sync::Arc,
        test_case::test_case,
        wasmer::{
            CompilerConfig, Engine, Instance, Module, RuntimeError, Singlepass, Store, Value,
            imports,
        },
        wasmer_middlewares::{Metering, metering::set_remaining_points},
    };

    const MOCK_WAT: &[u8] = br#"(module (memory (export "memory") 1))"#;

    const MOCK_BLOCK: BlockInfo = BlockInfo {
        height: 1,
        timestamp: Timestamp::from_nanos(100),
        hash: Hash256::ZERO,
    };

    fn setup_test(wat: &[u8], max_gas: Option<u64>) -> (Environment, Store, Box<Instance>) {
        let (gas_checkpoint, gas_tracker) = if let Some(max_gas) = max_gas {
            (max_gas, GasTracker::new_limited(max_gas))
        } else {
            (u64::MAX, GasTracker::new_limitless())
        };

        // Compile the contract; create Wasmer store and instance.
        let (store, instance) = {
            let mut compiler = Singlepass::new();
            compiler.push_middleware(Arc::new(Metering::new(0, |_| GAS_PER_OPERATION)));

            let engine = Engine::from(compiler);
            let module = Module::new(&engine, wat).unwrap();

            let mut store = Store::new(engine);

            let instance = Instance::new(&mut store, &module, &imports! {}).unwrap();
            let instance = Box::new(instance);

            set_remaining_points(&mut store, &instance, gas_checkpoint);

            (store, instance)
        };

        // Create the function environment.
        //
        // For production (in `WasmVm::build_instance`) this needs to be done
        // before creating the instance, but here for testing purpose, we don't
        // need any import function, so this can be done later, which is simpler.
        let env = {
            let storage = Shared::new(MockStorage::new());
            let storage_provider = StorageProvider::new(Box::new(storage.clone()), &[b"prefix"]);

            let querier_provider = QuerierProviderImpl::new_boxed(
                WasmVm::new(0),
                Box::new(storage),
                gas_tracker.clone(),
                MOCK_BLOCK,
            );

            let mut env = Environment::new(
                storage_provider,
                true,
                querier_provider,
                10,
                gas_tracker,
                gas_checkpoint,
            );

            env.set_wasmer_memory(&instance).unwrap();
            env.set_wasmer_instance(&instance).unwrap();
            env
        };

        (env, store, instance)
    }

    // 0 in - 1 out
    #[test_case(
        "0_1",
        vec![],
        Ok(vec![42]),
        1;
        "0 in 1 out: ok"
    )]
    #[test_case(
        "0_1",
        vec![],
        Err(VmError::ReturnCount { name: "0_1".to_string(), expect: 0, actual: 1 }),
        0;
        "0 in - 1 out: fails return count"
    )]
    #[test_case(
        "0_1",
        vec![1],
        Err(VmError::Runtime(RuntimeError::new("Parameters of type [I32] did not match signature [] -> [I32]"))),
        1;
        "0 in - 1 out: fails invalid signature"
    )]
    // 1 in - 0 out
    #[test_case(
        "1_0",
        vec![20],
        Ok(vec![]),
        0;
        "1 in - 0 out: ok"
    )]
    #[test_case(
        "1_0",
        vec![20],
        Err(VmError::ReturnCount { name: "1_0".to_string(), expect: 1, actual: 0 }),
        1;
        "1 in - 0 out: fails return count"
    )]
    // 1 in - 1 out
    #[test_case(
        "1_1",
        vec![20],
        Ok(vec![40]),
        1;
        "1 in - 1 out: ok"
    )]
    #[test_case(
        "1_1",
        vec![20],
        Err(VmError::ReturnCount { name: "1_1".to_string(), expect: 0, actual: 1 }),
        0;
        "1 in - 1 out: fails return count"
    )]
    // 2 in 1 out
    #[test_case(
        "2_1",
        vec![20, 22],
        Ok(vec![42]),
        1;
        "2 in 1 out: ok"
    )]
    #[test_case(
        "2_1",
        vec![20, 22],
        Err(VmError::ReturnCount { name: "2_1".to_string(), expect: 0, actual: 1 }),
        0;
        "2 in 1 out: fails return count"
    )]
    fn call_functions(
        name: &'static str,
        args: Vec<i32>,
        output: VmResult<Vec<i32>>,
        output_args: i8,
    ) {
        let args: Vec<Value> = args.into_iter().map(Into::into).collect();

        let wat = br#"
          (module
            (memory (export "memory") 1)
            ;; 0 in - 1 out
            (func $0_1 (result i32)
              i32.const 42)
            ;; 1 in - 1 out
            (func $1_1 (param i32) (result i32)
              local.get 0
              i32.const 2
              i32.mul)
            ;; 1 in - 0 out
            (func $1_0 (param i32)
              local.get 0
              drop)
            ;; 2 in - 1 out
            (func $2_1 (param i32) (param i32) (result i32)
              local.get 0
              local.get 1
              i32.add)
            ;; Exports
            (export "0_1" (func $0_1))
            (export "1_1" (func $1_1))
            (export "1_0" (func $1_0))
            (export "2_1" (func $2_1))
        )
        "#;

        let (mut env, mut store, _instance) = setup_test(wat, None);

        let result = match output_args {
            0 => env.call_function0(&mut store, name, &args).map(|_| vec![]),
            1 => env
                .call_function1(&mut store, name, &args)
                .map(|val| vec![val.i32().unwrap()]),
            _ => panic!("Invalid number of arguments"),
        };

        match (&result, &output) {
            (Ok(result), Ok(output)) => {
                assert_eq!(result, output);
            },
            (Err(result), Err(output)) => {
                assert_eq!(result.to_string(), output.to_string());
            },
            _ => panic!("Mismatched results: \n{:?}, \n{:?}", result, output),
        }
    }

    #[test]
    fn wasmer_gas_consumption() {
        let wat = br#"
        (module
          (memory (export "memory") 1)
          ;; Consume 5 gas, 4 gas per nop + 1 gas for end
          (func $consume_gas
            (nop)
            (nop)
            (nop)
            (nop)
          )
          (export "consume_gas" (func $consume_gas))
        )"#;

        let consume = |i, env: &mut Environment, store: &mut Store| -> VmResult<()> {
            for _ in 0..i {
                env.call_function0(store, "consume_gas", &[])?;
            }
            Ok(())
        };

        let (mut env, mut store, _instance) = setup_test(wat, Some(100));

        consume(1, &mut env, &mut store).unwrap();
        assert_eq!(env.gas_tracker.remaining(), Some(95));

        consume(10, &mut env, &mut store).unwrap();
        assert_eq!(env.gas_tracker.remaining(), Some(45));

        consume(9, &mut env, &mut store).unwrap();
        assert_eq!(env.gas_tracker.remaining(), Some(0));

        assert!(matches!(
            consume(1, &mut env, &mut store).unwrap_err(),
            VmError::Std(StdError::OutOfGas {
                limit: 100,
                used: 100,
                comment: "consume_gas",
            })
        ));
    }

    #[test]
    fn external_gas_consumption() {
        let (mut env, mut store, _instance) = setup_test(MOCK_WAT, Some(100));

        env.consume_external_gas(&mut store, 10, "comment").unwrap();
        assert_eq!(env.gas_tracker.remaining(), Some(90));

        env.consume_external_gas(&mut store, 90, "comment").unwrap();
        assert_eq!(env.gas_tracker.remaining(), Some(0));

        let err = env
            .consume_external_gas(&mut store, 1, "comment")
            .unwrap_err();

        assert!(matches!(
            err,
            VmError::Std(StdError::OutOfGas {
                limit: 100,
                used: 101,
                comment: "comment",
            })
        ));
    }

    // --- Ascending ---
    #[test_case(
        Iterator::new(None, None, Order::Ascending),
        &[
            Some((b"foo1", b"bar1")),
            Some((b"foo2", b"bar2")),
            Some((b"foo3", b"bar3")),
            None
        ];
        "no bound ascending"
    )]
    #[test_case(
        Iterator::new(Some(b"foo2".to_vec()), None, Order::Ascending),
        &[
            Some((b"foo2", b"bar2")),
            Some((b"foo3", b"bar3")),
            None
        ];
        "min bound ascending"
    )]
    #[test_case(
        Iterator::new(None, Some(b"foo3".to_vec()), Order::Ascending),
        &[
            Some((b"foo1", b"bar1")),
            Some((b"foo2", b"bar2")),
            None
        ];
        "max bound ascending"
    )]
    #[test_case(
        Iterator::new(Some(b"foo2".to_vec()), Some(b"foo3".to_vec()), Order::Ascending),
        &[
            Some((b"foo2", b"bar2")),
            None
        ];
        "min max bound ascending"
    )]
    // --- Descending ---
    #[test_case(
        Iterator::new(None, None, Order::Descending),
        &[
            Some((b"foo3", b"bar3")),
            Some((b"foo2", b"bar2")),
            Some((b"foo1", b"bar1")),
            None
        ];
        "no bound descending"
    )]
    #[test_case(
        Iterator::new(Some(b"foo2".to_vec()), None, Order::Descending),
        &[
            Some((b"foo3", b"bar3")),
            Some((b"foo2", b"bar2")),
            None
        ];
        "min bound descending"
    )]
    #[test_case(
        Iterator::new(None, Some(b"foo3".to_vec()), Order::Descending),
        &[
            Some((b"foo2", b"bar2")),
            Some((b"foo1", b"bar1")),
            None
        ];
        "max bound descending"
    )]
    #[test_case(
        Iterator::new(Some(b"foo2".to_vec()), Some(b"foo3".to_vec()), Order::Descending),
        &[
            Some((b"foo2", b"bar2")),
            None
        ];
        "min max bound descending"
    )]
    fn iterator(iterator: Iterator, read: &[Option<(&[u8], &[u8])>]) {
        let (mut env, ..) = setup_test(MOCK_WAT, None);

        env.storage.write(b"foo1", b"bar1");
        env.storage.write(b"foo2", b"bar2");
        env.storage.write(b"foo3", b"bar3");

        env.add_iterator(iterator);

        for assert in read {
            let result = env.advance_iterator(0).unwrap();
            assert_eq!(result, assert.map(|(k, v)| { (k.to_vec(), v.to_vec()) }));
        }
    }

    #[test]
    fn multiple_iterators() {
        let (mut env, ..) = setup_test(MOCK_WAT, None);

        let advance_and_assert = |env: &mut Environment, id, expect: Option<(&[u8], &[u8])>| {
            let record = env.advance_iterator(id).unwrap();
            assert_eq!(record, expect.map(|(k, v)| (k.to_vec(), v.to_vec())));
        };

        env.storage.write(b"foo1", b"bar1");
        env.storage.write(b"foo2", b"bar2");
        env.storage.write(b"foo3", b"bar3");

        let iterator = Iterator::new(None, None, Order::Ascending);
        let iter_id_0 = env.add_iterator(iterator);
        assert_eq!(iter_id_0, 0);

        advance_and_assert(&mut env, 0, Some((b"foo1", b"bar1")));

        let iterator = Iterator::new(Some(b"foo1".to_vec()), None, Order::Ascending);
        let iter_id_1 = env.add_iterator(iterator);
        assert_eq!(iter_id_1, 1);

        advance_and_assert(&mut env, 0, Some((b"foo2", b"bar2")));
        advance_and_assert(&mut env, 1, Some((b"foo1", b"bar1")));
        advance_and_assert(&mut env, 0, Some((b"foo3", b"bar3")));
        advance_and_assert(&mut env, 0, None);
        advance_and_assert(&mut env, 0, None);

        // Add another key-value pair to the storage
        env.storage.write(b"foo4", b"bar4");

        advance_and_assert(&mut env, 0, Some((b"foo4", b"bar4")));
        advance_and_assert(&mut env, 1, Some((b"foo2", b"bar2")));

        // Clear iterators
        env.clear_iterators();

        env.advance_iterator(0).unwrap_err();

        let iterator = Iterator::new(None, None, Order::Ascending);
        let iter_id_2 = env.add_iterator(iterator);
        assert_eq!(iter_id_2, 2);
    }
}
