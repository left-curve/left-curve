use host::setup_instance;

fn main() -> anyhow::Result<()> {
    let mut instance = setup_instance::<_, ()>(
        "../../target/wasm32-unknown-unknown/release/greeter.wasm",
        None,
    )?;
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

    println!("Wasm module responds: {greeting}");

    // deallocate the response data
    // no need to deallocate the name, it's already freed in Wasm code
    instance.call("deallocate", greeting_region_ptr)?;

    Ok(())
}
