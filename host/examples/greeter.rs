use {
    host::InstanceBuilder,
    std::{env, path::PathBuf},
};

const NAME: &str = "Larry";

fn main() -> anyhow::Result<()> {
    let wasm_file = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?)
        .join("../target/wasm32-unknown-unknown/debug/greeter.wasm");
    let mut instance = InstanceBuilder::default()
        .with_wasm_file(wasm_file)?
        .with_host_state(())
        .finalize()?;

    // allocate a region in the Wasm memory and put the name bytes into it
    // let name_region_ptr = instance.call("allocate", name_bytes.capacity() as u32)?;
    // instance.write_region(name_region_ptr, &name_bytes)?;
    let name_ptr = instance.release_buffer(NAME.as_bytes().to_vec())?;

    // call the hello function
    // then, fetch the response data from Wasm memory
    let greeting_ptr = instance.call("hello", name_ptr)?;
    let greeting_bytes = instance.consume_region(greeting_ptr)?;
    let greeting = String::from_utf8(greeting_bytes)?;

    println!("Wasm module responds: {greeting}");

    Ok(())
}
