mod hybrid;
mod wasm;

const WASM_CACHE_CAPACITY: usize = 10;

fn read_wasm_file(filename: &str) -> dango_primitives::Binary {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = format!("{manifest_dir}/../core/vm/wasm/testdata/{filename}");
    std::fs::read(path).unwrap().into()
}
