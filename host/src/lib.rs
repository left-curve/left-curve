mod instance;
mod region;

pub use crate::{instance::Instance, region::Region};

use {
    std::{fs::File, path::Path},
    wasmi::{Engine, Linker, Module, Store},
};

pub fn setup_instance<P, T>(
    wasm_file:  P,
    maybe_data: Option<T>,
) -> anyhow::Result<Instance<T>>
where
    P: AsRef<Path>,
    T: Default,
{
    // create wasmi interpreter engine with default configuration
    let engine = Engine::default();

    // read wasm binary from file and create module
    let mut file = File::open(wasm_file)?;
    let module = Module::new(&engine, &mut file)?;

    // create store, and define import functions
    let mut store = Store::new(&engine, maybe_data.unwrap_or_default());
    let linker = <Linker<T>>::new(&engine);

    // define import functions
    // linker.define("env", "db_read", Func::wrap(&mut store, |caller: Caller<'_, T>, param: i32| {
    //     let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
    //     let mut buf = vec![0x8; 1];
    //     memory.read(&caller, 0, &mut buf).unwrap();
    //     // let number = caller.data().get(k)
    // }))?;
    // linker.define("env", "db_write", Func::wrap(&mut store, |caller: Caller<'_, T>, param: i32, result: i32| {
    //     todo!();
    // }))?;
    // linker.define("env", "db_remove", Func::wrap(&mut store, |caller: Caller<'_, T>, param: i32| {
    //     todo!();
    // }))?;

    // create the Wasm instance
    let instance = linker.instantiate(&mut store, &module)?.start(&mut store)?;

    Ok(Instance { instance, store })
}
