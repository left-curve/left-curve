use std::mem;

/// Reserve a region in Wasm memory of the given number of bytes. Return the
/// memory address of a Region object that describes the memory region that was
/// reserved.
///
/// This is used by the host to pass non-primitive data into the Wasm module.
#[no_mangle]
extern "C" fn allocate(capacity: usize) -> usize {
    let data = Vec::<u8>::with_capacity(capacity);
    Region::release_buffer(data) as usize
}

/// Free the specified region in the Wasm module's linear memory.
#[no_mangle]
extern "C" fn deallocate(region_addr: usize) {
    let _ = unsafe { Region::consume(region_addr as *mut Region) };
    // data is dropped here, which calls Vec<u8> destructor, freeing the memory
}

/// Describes a region in the Wasm linear memory.
#[repr(C)]
pub struct Region {
    offset:   usize,
    capacity: usize,
    length:   usize,
}

impl Region {
    /// Consume an existing vector data, returns a pointer to the Region.
    pub fn release_buffer(data: Vec<u8>) -> *mut Self {
        let region = Box::new(Self {
            offset:   data.as_ptr() as usize,
            capacity: data.capacity(),
            length:   data.len(),
        });

        // drop the `data` value without freeing the memory
        mem::forget(data);

        // return the memory address of the Region, without freeing the memory
        Box::into_raw(region)
    }

    /// The reverse of `release_buffer`. Consume a pointer to a Region, return
    /// the vector data.
    pub unsafe fn consume(ptr: *mut Region) -> Vec<u8> {
        assert!(!ptr.is_null(), "Region pointer is null");

        let region = Box::from_raw(ptr);
        let region_start = region.offset as *mut u8;
        assert!(!region_start.is_null(), "Region starts as null address");

        Vec::from_raw_parts(region_start, region.length, region.capacity)
    }
}
