mod instance;
mod region;

pub use crate::{instance::Instance, region::Region};

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        crate::Instance,
        std::fs::File,
        wasmi::{Engine, Linker, Module, Store},
    };

    fn setup_test<T: Default>() -> anyhow::Result<Instance<T>> {
        // create wasmi interpreter engine with default configuration
        let engine = Engine::default();

        // read wasm binary from file and create module
        let mut file = File::open("../target/wasm32-unknown-unknown/release/guest.wasm")?;
        let module = Module::new(&engine, &mut file)?;

        // create store, and define import functions
        let mut store = Store::new(&engine, T::default());
        let linker = <Linker<T>>::new(&engine);

        // if the host provides any import functions, define them here
        // in this particular example, we don't have any
        // ...

        // create the Wasm instance
        let instance = linker.instantiate(&mut store, &module)?.start(&mut store)?;

        Ok(Instance { instance, store })
    }

    #[test]
    fn add() -> anyhow::Result<()> {
        let mut instance = setup_test()?;

        const A: u32 = 123;
        const B: u32 = 456;
        let sum: u32 = instance.call("add", (A, B))?;

        assert_eq!(sum, A + B);

        Ok(())
    }

    #[test]
    fn hello() -> anyhow::Result<()> {
        let mut instance = setup_test()?;

        const NAME: &str = "Larry";
        let name_bytes = NAME.as_bytes().to_vec();

        // allocate a region in the Wasm memory and put the name bytes into it
        let name_region_ptr = instance.call("allocate", name_bytes.capacity() as u32)?;
        instance.write_region(name_region_ptr, &name_bytes)?;

        // call the hello function
        let greeting_region_ptr = instance.call("hello", name_region_ptr)?;

        // fetch the response data from Wasm memory
        let greeting_bytes = instance.read_region(greeting_region_ptr)?;
        let greeting = String::from_utf8(greeting_bytes)?;

        assert_eq!(greeting, format!("Hello {NAME}!"));

        // deallocate the response data
        // no need to deallocate the name, it's already freed in Wasm code
        instance.call("deallocate", greeting_region_ptr)?;

        Ok(())
    }
}
