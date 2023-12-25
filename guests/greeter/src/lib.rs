use sdk::Region;

/// Read a string, and return string that reads "Hello {input}!".
///
/// E.g. if input is `"Larry"`, the output should be `"Hello Larry!"`.
///
/// The input and output of this function are strings, which are not primitive
/// types. Therefore we have to dynamically allocate them into the Wasm module's
/// linear memory, and pass pointers over the FFI boundary.
#[no_mangle]
pub extern "C" fn hello(region_addr: usize) -> usize {
    // use Region::consume to read a byte slice that has been loaded into the
    // Wasm memory by the host
    let name_bytes = unsafe { Region::consume(region_addr as *mut Region) };

    // attempt to parse the input data into a String
    let name = String::from_utf8(name_bytes).unwrap_or_else(|err| {
        panic!("Failed to parse name from utf8: {err}");
    });

    // say hello
    let greeting = format!("Hello {name}!");

    // use Region::release_buffer to return the response to the host
    Region::release_buffer(greeting.into_bytes()) as usize
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greeting() {
        let name = "Larry".as_bytes().to_vec();

        // host uses Region::release_buffer to load the input data into Wasm
        // memory
        let ptr = Region::release_buffer(name);

        // host calls guest function, gets a memory address back
        let addr = hello(ptr as usize);

        // host uses Region::consume to read the byte slice returned by guest
        let bytes = unsafe { Region::consume(addr as *mut Region) };

        // attempt to parse the response into a String
        let greeting = String::from_utf8(bytes).unwrap();

        assert_eq!(greeting, "Hello Larry!");
    }
}
