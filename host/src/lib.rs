mod instance;
mod region;

pub use crate::{instance::Instance, region::Region};

use {
    anyhow::Context,
    std::{fs::File, path::Path},
    wasmi::{Engine, Func, IntoFunc, Linker, Module, Store},
};

#[derive(Default)]
pub struct InstanceBuilder<HostState> {
    engine: Engine,
    module: Option<Module>,
    store:  Option<Store<HostState>>,
    linker: Option<Linker<HostState>>,
}

impl<HostState> InstanceBuilder<HostState> {
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
        let mut store = self.take_store()?;
        let mut linker = self.take_linker()?;

        linker.define("env", name, Func::wrap(&mut store, func))?;

        self.store = Some(store);
        self.linker = Some(linker);

        Ok(self)
    }

    pub fn finalize(mut self) -> anyhow::Result<Instance<HostState>> {
        let module = self.take_module()?;
        let mut store = self.take_store()?;
        let linker = self.take_linker()?;
        let instance = linker.instantiate(&mut store, &module)?.start(&mut store)?;

        Ok(Instance { instance, store })
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
