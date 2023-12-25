use {
    host::InstanceBuilder,
    std::{env, path::PathBuf},
};

fn main() -> anyhow::Result<()> {
    let wasm_file = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?)
        .join("../target/wasm32-unknown-unknown/debug/add.wasm");
    let mut instance = InstanceBuilder::default()
        .with_wasm_file(wasm_file)?
        .with_host_state(())
        .finalize()?;

    const A: u32 = 123;
    const B: u32 = 456;
    let sum: u32 = instance.call("add", (A, B))?;

    println!("Wasm module responds: {A} + {B} = {sum}");

    Ok(())
}
