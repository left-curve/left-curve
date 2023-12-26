mod alloc;
mod errors;
mod instance;
mod memory;

pub use crate::{alloc::Allocator, errors::Error, instance::Host, memory::{Memory, Region}};

use {
    anyhow::Context,
    std::{fs::File, path::Path},
    wasmi::{Engine, IntoFunc, Linker, Module, Store},
};

#[derive(Default)]
pub struct HostBuilder<HostState> {
    engine: Engine,
    module: Option<Module>,
    store:  Option<Store<HostState>>,
    linker: Option<Linker<HostState>>,
}

impl<HostState> HostBuilder<HostState> {
    pub fn new(engine: Engine) -> Self {
        Self {
            engine,
            module: None,
            store:  None,
            linker: None,
        }
    }

    pub fn with_wasm_file(mut self, path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let mut file = File::open(path)?;
        self.module = Some(Module::new(&self.engine, &mut file)?);
        Ok(self)
    }

    pub fn with_host_state(mut self, data: HostState) -> Self {
        self.store = Some(Store::new(&self.engine, data));
        self.linker = Some(Linker::new(&self.engine));
        self
    }

    pub fn with_host_function<Params, Results>(
        mut self,
        name: &str,
        func: impl IntoFunc<HostState, Params, Results>,
    ) -> anyhow::Result<Self> {
        let mut linker = self.take_linker()?;
        linker.func_wrap("env", name, func)?;
        self.linker = Some(linker);

        Ok(self)
    }

    pub fn finalize(mut self) -> anyhow::Result<Host<HostState>> {
        let module = self.take_module()?;
        let mut store = self.take_store()?;
        let linker = self.take_linker()?;
        let instance = linker.instantiate(&mut store, &module)?.start(&mut store)?;

        Host::new(instance, store)
    }

    fn take_module(&mut self) -> anyhow::Result<Module> {
        self.module.take().context("Module not yet initialized")
    }

    fn take_store(&mut self) -> anyhow::Result<Store<HostState>> {
        self.store.take().context("Store not yet initialized")
    }

    fn take_linker(&mut self) -> anyhow::Result<Linker<HostState>> {
        self.linker.take().context("Linker not yet initialized")
    }
}
