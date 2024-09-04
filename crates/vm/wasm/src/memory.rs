use {
    crate::{Environment, Region, UncheckedRegion, VmError, VmResult},
    data_encoding::BASE64,
    wasmer::{AsStoreMut, AsStoreRef, MemoryView, WasmPtr},
};

pub fn read_from_memory<S>(env: &mut Environment, store: &S, region_ptr: u32) -> VmResult<Vec<u8>>
where
    S: AsStoreRef,
{
    let memory = env.get_wasmer_memory(&store)?;

    // read region
    let region = read_region(&memory, region_ptr)?;

    // read memory area indicated by region
    let mut buf = vec![0u8; region.length as usize];
    memory.read(region.offset as u64, &mut buf)?;

    Ok(buf)
}

pub fn read_then_wipe<S>(env: &mut Environment, store: &mut S, region_ptr: u32) -> VmResult<Vec<u8>>
where
    S: AsStoreMut,
{
    let data = read_from_memory(env, store, region_ptr)?;
    env.call_function0(store, "deallocate", &[region_ptr.into()])?;

    Ok(data)
}

pub fn write_to_memory<S>(env: &mut Environment, store: &mut S, data: &[u8]) -> VmResult<u32>
where
    S: AsStoreMut,
{
    // call the `allocate` export to reserve an area in Wasm memory
    let region_ptr: u32 = env
        .call_function1(store, "allocate", &[(data.len() as u32).into()])?
        .try_into()
        .map_err(VmError::ReturnType)?;
    let memory = env.get_wasmer_memory(&store)?;
    let mut region = read_region(&memory, region_ptr)?;
    // don't forget to update region length
    region.length = data.len() as u32;

    if region.length > region.capacity {
        return Err(VmError::RegionTooSmall {
            offset: region.offset,
            capacity: region.capacity,
            data: BASE64.encode(data),
        });
    }

    // write the data to the reserved area
    memory.write(region.offset as u64, data)?;

    // write the Region
    write_region(&memory, region_ptr, region)?;

    Ok(region_ptr)
}

fn read_region(memory: &MemoryView, offset: u32) -> VmResult<Region> {
    let wptr = <WasmPtr<UncheckedRegion>>::new(offset);
    wptr.deref(memory).read()?.try_into()
}

fn write_region(memory: &MemoryView, offset: u32, region: Region) -> VmResult<()> {
    let wptr = <WasmPtr<Region>>::new(offset);
    wptr.deref(memory).write(region).map_err(Into::into)
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {

    use {super::*, std::mem, test_case::test_case};

    #[test]
    fn region_has_known_size() {
        // 3x4 bytes with no padding
        assert_eq!(mem::size_of::<Region>(), 12);
    }

    #[test_case(
        UncheckedRegion {
            offset: 23,
            capacity: 500,
            length: 0,
        }; "empty"
    )]
    #[test_case(
        UncheckedRegion {
            offset: 23,
            capacity: 500,
            length: 250,
        }; "half full"
    )]
    #[test_case(
        UncheckedRegion {
            offset: 23,
            capacity: 500,
            length: 500,
        }; "full"
    )]
    #[test_case(
        UncheckedRegion {
            offset: u32::MAX,
            capacity: 0,
            length: 0,
        }; "end of linear memory 1"
    )]
    #[test_case(
        UncheckedRegion {
            offset: 1,
            capacity: u32::MAX - 1,
            length: 0,
        }; "end of linear memory 2"
    )]
    fn valid_regions(region: UncheckedRegion) {
        Region::try_from(region).unwrap();
    }

    #[test_case(
        UncheckedRegion {
            offset: 0,
            capacity: 500,
            length: 250,
        },
        |error| {
            assert!(matches!(error, VmError::RegionZeroOffset));
        };
        "zero offset"
    )]
    #[test_case(
        UncheckedRegion {
            offset: 23,
            capacity: 500,
            length: 501,
        },
        |error| {
            assert!(matches!(error, VmError::RegionLengthExceedsCapacity { length, capacity } if length == 501 && capacity == 500));
        };
        "length exceeding capacity"
    )]
    #[test_case(
        UncheckedRegion {
            offset: 23,
            capacity: u32::MAX,
            length: 501,
        },
        |error| {
            assert!(matches!(error, VmError::RegionOutOfRange { offset, capacity } if offset == 23 && capacity == u32::MAX));
        };
        "exceeding address space 1"
    )]
    #[test_case(
        UncheckedRegion {
            offset: u32::MAX,
            capacity: 1,
            length: 0,
        },
        |error| {
            assert!(matches!(error, VmError::RegionOutOfRange { offset, capacity } if offset == u32::MAX && capacity == 1));
        };
        "exceeding address space 2"
    )]
    fn unvalid_regions<F>(region: UncheckedRegion, callback: F)
    where
        F: Fn(VmError),
    {
        let error = Region::try_from(region).unwrap_err();
        callback(error);
    }
}
