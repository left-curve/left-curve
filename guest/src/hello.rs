use crate::memory::Region;

/// Read a string, and return string that reads "Hello {input}!".
///
/// E.g. if input is `"Larry"`, the output should be `"Hello Larry!"`.
///
/// The input and output of this function are strings, which are not primitive
/// types. Therefore we have to dynamically allocate them onto the Wasm module's
/// linear memory, and pass pointers over the FFI boundary.
#[no_mangle]
pub extern "C" fn hello(region_addr: usize) -> usize {
    let name_bytes = unsafe { Region::consume(region_addr as *mut Region) };
    let name = String::from_utf8(name_bytes).unwrap_or_else(|err| {
        panic!("Failed to parse name from utf8: {err}");
    });

    let greeting = format!("Hello {name}!");
    let greeting_bytes = greeting.into_bytes();

    Region::release_buffer(greeting_bytes) as usize
}
