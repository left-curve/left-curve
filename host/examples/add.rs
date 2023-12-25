use host::setup_instance;

fn main() -> anyhow::Result<()> {
    let mut instance = setup_instance::<_, ()>(
        "../../target/wasm32-unknown-unknown/release/add.wasm",
        None,
    )?;

    const A: u32 = 123;
    const B: u32 = 456;
    let sum: u32 = instance.call("add", (A, B))?;

    println!("Wasm module responds: {A} + {B} = {sum}");

    Ok(())
}
