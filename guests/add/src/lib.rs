/// Read two numbers and return their sum.
///
/// The input and output of this function only involve primitive types that can
/// be pushed onto the Wasm stack, so we don't need to worry about dynamically
/// allocating memories.
#[no_mangle]
extern "C" fn add(a: usize, b: usize) -> usize {
    a + b
}
