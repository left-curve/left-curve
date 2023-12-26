use {
    host::{Host, InstanceBuilder},
    std::{env, path::PathBuf},
};

fn main() -> anyhow::Result<()> {
    let wasm_file = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?)
        .join("../target/wasm32-unknown-unknown/debug/add.wasm");
    let (instance, mut store) = InstanceBuilder::default()
        .with_wasm_file(wasm_file)?
        .with_host_state(())
        .finalize()?;
    let mut host = Host::new(&instance, &mut store);

    const A: u32 = 123;
    const B: u32 = 456;
    let sum: u32 = host.call("add", (A, B))?;

    println!("Wasm module responds: {A} + {B} = {sum}");

    Ok(())
}
