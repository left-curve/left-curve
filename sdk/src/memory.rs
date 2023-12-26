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
    /// Build a Region describing an existing byte slice.
    ///
    /// IMPORTANT MEMORY SAFETY NOTE:
    /// This function does NOT take ownership of `data`, therefore the host must
    /// NOT call `deallocate` to free the memory.
    pub fn build(data: &[u8]) -> Box<Region> {
        Box::new(Self {
            offset:   data.as_ptr() as usize,
            capacity: data.len(),
            length:   data.len(),
        })
    }

    /// Consume an existing vector data, returns a pointer to the Region.
    ///
    /// Pretty much the only use case for this is to return data to the host
    /// at the very end of the call. For all other use cases, Region::build
    /// probably should be used.
    ///
    /// IMPORTANT MEMORY SAFETY NOTE:
    /// The variable `data` is dropped, but the memory it takes is not freed.
    /// The host MUST call the `deallocate` export to free the memory spaces
    /// taken by _both_ the Region and the Vec.
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

    /// Typically used by the guest to read data provide by the host.
    ///
    /// NOTE: memory space taken by the Region is freed; memory space referenced
    /// by the Region has its ownership captured by the Vec.
    pub unsafe fn consume(ptr: *mut Region) -> Vec<u8> {
        assert!(!ptr.is_null(), "Region pointer is null");

        let region = Box::from_raw(ptr);
        let region_start = region.offset as *mut u8;
        assert!(!region_start.is_null(), "Region starts as null address");

        Vec::from_raw_parts(region_start, region.length, region.capacity)
    }
}
