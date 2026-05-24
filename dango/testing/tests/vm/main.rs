mod hybrid;
mod wasm;

const WASM_CACHE_CAPACITY: usize = 10;

fn read_wasm_file(filename: &str) -> grug_types::Binary {
    let path = format!("{}/testdata/{filename}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(path).unwrap().into()
}
