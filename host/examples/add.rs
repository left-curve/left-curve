use {
    host::setup_instance,
    std::{env, path::PathBuf},
};

fn main() -> anyhow::Result<()> {
    let wasm_file = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?)
        .join("../target/wasm32-unknown-unknown/debug/add.wasm");
    let mut instance = setup_instance::<_, ()>(wasm_file, None)?;

    const A: u32 = 123;
    const B: u32 = 456;
    let sum: u32 = instance.call("add", (A, B))?;

    println!("Wasm module responds: {A} + {B} = {sum}");

    Ok(())
}
