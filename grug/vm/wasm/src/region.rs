use {crate::VmError, std::mem, wasmer::ValueType};

/// Descriptor of a region in a Wasm instance's memory.
///
/// In Grug's Wasm VM, this is used for calling a contract with dynamically-sized
/// input data. To achieve this,
///
/// - The host first calls the contract's `allocate` function, informing the
///   contract how many bytes of memory needs to be reserved.
/// - The contract allocates a region of such a size in its heap memory. Then,
///   it creates a `grug_ffi::Region` instance that describes this region,
///   and return the pointer (i.e. memory address) to this descriptor.
/// - The host dereferences the pointer into a `grug_vm_wasm::Region` instance,
///   and loads the input data into the region it describes.
///
/// ## Note
///
/// Do not confuse `grug_ffi::Region` and `grug_vm_wasm::Region`!
///
/// - The former is used by the contract, and uses `usize` to represent memory
///   addresses.
/// - The latter is used by the host; since the host knows the Wasm runtime is
///   32-bit, it uses `u32` to represent memory addresses.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Region {
    // The fields are made private, such that a `Region` instance can't be
    // created as a struct literal. Instead, only create it by validating an `UncheckedRegion`.
    pub(crate) offset: u32,
    pub(crate) capacity: u32,
    pub(crate) length: u32,
}

unsafe impl ValueType for Region {
    fn zero_padding_bytes(&self, _bytes: &mut [mem::MaybeUninit<u8>]) {
        // There is no padding in Region so not necessary.
    }
}

/// Descriptor of a region in a Wasm instance's memory, but its validity
/// undetermined.
///
/// The descriptor can be invalid if:
/// - its `offset` is zero;
/// - its `length` is longer than its `capacity`;
/// - the sum of its `offset` and `capacity` is greater than `u32::MAX`.
///
/// Since smart contracts are potentially malicious, we can't trust the region
/// descriptor it returns. Instead, we dereference the return value into an
/// `UncheckedRegion` and perform necessary checks, throwing error if necessary.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UncheckedRegion {
    // The fields are made private, such that an `UncheckedRegion` instance can't
    // be created as a struct literal. Instead, only create it by dereferencing
    // a pointer from Wasm memory.
    offset: u32,
    capacity: u32,
    length: u32,
}

unsafe impl ValueType for UncheckedRegion {
    fn zero_padding_bytes(&self, _bytes: &mut [mem::MaybeUninit<u8>]) {
        // There is no padding in Region so not necessary.
    }
}

impl TryFrom<UncheckedRegion> for Region {
    type Error = VmError;

    fn try_from(unchecked: UncheckedRegion) -> Result<Self, Self::Error> {
        if unchecked.offset == 0 {
            return Err(VmError::region_zero_offset());
        }

        if unchecked.length > unchecked.capacity {
            return Err(VmError::region_length_exceeds_capacity(
                unchecked.length,
                unchecked.capacity,
            ));
        }

        if unchecked.capacity > (u32::MAX - unchecked.offset) {
            return Err(VmError::region_out_of_range(
                unchecked.offset,
                unchecked.capacity,
            ));
        }

        Ok(Region {
            offset: unchecked.offset,
            capacity: unchecked.capacity,
            length: unchecked.length,
        })
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        crate::{Region, UncheckedRegion, VmError, VmResult},
        std::mem,
        test_case::test_case,
    };

    #[test]
    fn region_has_known_size() {
        // 3x4 bytes with no padding
        assert_eq!(mem::size_of::<Region>(), 12);
        assert_eq!(mem::size_of::<UncheckedRegion>(), 12);
    }

    #[test_case(
        UncheckedRegion {
            offset: 23,
            capacity: 500,
            length: 0,
        },
        |res| {
            assert!(res.is_ok());
        };
        "empty"
    )]
    #[test_case(
        UncheckedRegion {
            offset: 23,
            capacity: 500,
            length: 250,
        },
        |res| {
            assert!(res.is_ok());
        };
        "half full"
    )]
    #[test_case(
        UncheckedRegion {
            offset: 23,
            capacity: 500,
            length: 500,
        },
        |res| {
            assert!(res.is_ok());
        };
        "full"
    )]
    #[test_case(
        UncheckedRegion {
            offset: u32::MAX,
            capacity: 0,
            length: 0,
        },
        |res| {
            assert!(res.is_ok());
        };
        "end of linear memory 1"
    )]
    #[test_case(
        UncheckedRegion {
            offset: 1,
            capacity: u32::MAX - 1,
            length: 0,
        },
        |res| {
            assert!(res.is_ok());
        };
        "end of linear memory 2"
    )]
    #[test_case(
        UncheckedRegion {
            offset: 0,
            capacity: 500,
            length: 250,
        },
        |res| {
            assert!(matches!(res, Err(VmError::RegionZeroOffset {..})));
        };
        "zero offset"
    )]
    #[test_case(
        UncheckedRegion {
            offset: 23,
            capacity: 500,
            length: 501,
        },
        |res| {
            assert!(matches!(res, Err(VmError::RegionLengthExceedsCapacity { length, capacity, .. }) if length == 501 && capacity == 500));
        };
        "length exceeding capacity"
    )]
    #[test_case(
        UncheckedRegion {
            offset: 23,
            capacity: u32::MAX,
            length: 501,
        },
        |res| {
            assert!(matches!(res, Err(VmError::RegionOutOfRange { offset, capacity, .. }) if offset == 23 && capacity == u32::MAX));
        };
        "exceeding address space 1"
    )]
    #[test_case(
        UncheckedRegion {
            offset: u32::MAX,
            capacity: 1,
            length: 0,
        },
        |res| {
            assert!(matches!(res, Err(VmError::RegionOutOfRange { offset, capacity, .. }) if offset == u32::MAX && capacity == 1));
        };
        "exceeding address space 2"
    )]
    fn validating_region<F>(region: UncheckedRegion, callback: F)
    where
        F: FnOnce(VmResult<Region>),
    {
        let res = Region::try_from(region);
        callback(res);
    }
}
