use {
    host::{Host, InstanceBuilder},
    std::{env, path::PathBuf},
};

const NAME: &str = "Larry";

fn main() -> anyhow::Result<()> {
    let wasm_file = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?)
        .join("../target/wasm32-unknown-unknown/debug/greeter.wasm");
    let (instance, mut store) = InstanceBuilder::default()
        .with_wasm_file(wasm_file)?
        .with_host_state(())
        .finalize()?;
    let mut host = Host::new(&instance, &mut store);

    // allocate a region in the Wasm memory and put the name bytes into it
    // let name_region_ptr = instance.call("allocate", name_bytes.capacity() as u32)?;
    // instance.write_region(name_region_ptr, &name_bytes)?;
    let name_ptr = host.write_to_memory(NAME.as_bytes())?;

    // call the hello function
    // then, fetch the response data from Wasm memory
    // NOTE that the response should be deallocated after reading
    let greeting_ptr = host.call("hello", name_ptr)?;
    let greeting_bytes = host.read_then_wipe(greeting_ptr)?;
    let greeting = String::from_utf8(greeting_bytes)?;

    println!("Wasm module responds: {greeting}");

    Ok(())
}
