use {
    crate::{Environment, Region, VmError, VmResult},
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
    let wptr = <WasmPtr<Region>>::new(offset);
    let region = wptr.deref(memory).read()?;
    validate_region(&region)?;
    Ok(region)
}

fn write_region(memory: &MemoryView, offset: u32, region: Region) -> VmResult<()> {
    let wptr = <WasmPtr<Region>>::new(offset);
    wptr.deref(memory).write(region).map_err(Into::into)
}

fn validate_region(region: &Region) -> VmResult<()> {
    if region.offset == 0 {
        return Err(VmError::RegionZeroOffset {});
    }
    if region.length > region.capacity {
        return Err(VmError::RegionLengthExceedsCapacity {
            length: region.length,
            capacity: region.capacity,
        });
    }
    if region.capacity > (u32::MAX - region.offset) {
        return Err(VmError::RegionOutOfRange {
            offset: region.offset,
            capacity: region.capacity,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {

    use {super::*, std::mem};

    #[test]
    fn region_has_known_size() {
        // 3x4 bytes with no padding
        assert_eq!(mem::size_of::<Region>(), 12);
    }

    #[test]
    fn validate_region_passes_for_valid_region() {
        // empty
        let region = Region {
            offset: 23,
            capacity: 500,
            length: 0,
        };
        validate_region(&region).unwrap();

        // half full
        let region = Region {
            offset: 23,
            capacity: 500,
            length: 250,
        };
        validate_region(&region).unwrap();

        // full
        let region = Region {
            offset: 23,
            capacity: 500,
            length: 500,
        };
        validate_region(&region).unwrap();

        // at end of linear memory (1)
        let region = Region {
            offset: u32::MAX,
            capacity: 0,
            length: 0,
        };
        validate_region(&region).unwrap();

        // at end of linear memory (2)
        let region = Region {
            offset: 1,
            capacity: u32::MAX - 1,
            length: 0,
        };
        validate_region(&region).unwrap();
    }

    #[test]
    fn validate_region_fails_for_zero_offset() {
        let region = Region {
            offset: 0,
            capacity: 500,
            length: 250,
        };
        let result = validate_region(&region);
        match result.unwrap_err() {
            VmError::RegionZeroOffset { .. } => {},
            e => panic!("Got unexpected error: {e:?}"),
        }
    }

    #[test]
    fn validate_region_fails_for_length_exceeding_capacity() {
        let region = Region {
            offset: 23,
            capacity: 500,
            length: 501,
        };
        let result = validate_region(&region);
        match result.unwrap_err() {
            VmError::RegionLengthExceedsCapacity {
                length, capacity, ..
            } => {
                assert_eq!(length, 501);
                assert_eq!(capacity, 500);
            },
            e => panic!("Got unexpected error: {e:?}"),
        }
    }

    #[test]
    fn validate_region_fails_when_exceeding_address_space() {
        let region = Region {
            offset: 23,
            capacity: u32::MAX,
            length: 501,
        };
        let result = validate_region(&region);
        match result.unwrap_err() {
            VmError::RegionOutOfRange {
                offset, capacity, ..
            } => {
                assert_eq!(offset, 23);
                assert_eq!(capacity, u32::MAX);
            },
            e => panic!("Got unexpected error: {e:?}"),
        }

        let region = Region {
            offset: u32::MAX,
            capacity: 1,
            length: 0,
        };
        let result = validate_region(&region);
        match result.unwrap_err() {
            VmError::RegionOutOfRange {
                offset, capacity, ..
            } => {
                assert_eq!(offset, u32::MAX);
                assert_eq!(capacity, 1);
            },
            e => panic!("Got unexpected error: {e:?}"),
        }
    }
}
