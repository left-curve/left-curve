use std::mem;

/// Describes a region in the Wasm linear memory.
#[repr(C)]
pub struct Region {
    offset: usize,
    capacity: usize,
    length: usize,
}

impl Region {
    /// Build a Region describing an existing byte slice.
    ///
    /// ## Important memory safety note:
    ///
    /// This function does NOT take ownership of `data`, therefore the host must
    /// NOT call `deallocate` to free the memory.
    pub fn build(data: &[u8]) -> Box<Region> {
        Box::new(Self {
            offset: data.as_ptr() as usize,
            capacity: data.len(),
            length: data.len(),
        })
    }

    /// Consume an existing vector data, returns a pointer to the Region.
    ///
    /// Pretty much the only use case for this is to return data to the host
    /// at the very end of the call. For all other use cases, Region::build
    /// probably should be used.
    ///
    /// ## Important memory safety note:
    ///
    /// The variable `data` is dropped, but the memory it takes is not freed.
    /// The host MUST call the `deallocate` export to free the memory spaces
    /// taken by _both_ the Region and the Vec.
    pub fn release_buffer(data: Vec<u8>) -> *mut Self {
        let region = Box::new(Self {
            offset: data.as_ptr() as usize,
            capacity: data.capacity(),
            length: data.len(),
        });

        // drop the `data` value without freeing the memory
        mem::forget(data);

        // return the memory address of the Region, without freeing the memory
        Box::into_raw(region)
    }

    /// Typically used by the guest to read data provide by the host.
    ///
    /// Note: Memory space taken by the Region is freed; memory space referenced
    /// by the Region has its ownership captured by the Vec.
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn consume(ptr: *mut Region) -> Vec<u8> {
        assert!(!ptr.is_null(), "Region pointer is null");

        let region = unsafe { Box::from_raw(ptr) };
        let region_start = region.offset as *mut u8;
        assert!(!region_start.is_null(), "Region starts as null address");

        unsafe { Vec::from_raw_parts(region_start, region.length, region.capacity) }
    }
}
