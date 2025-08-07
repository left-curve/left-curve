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
        .map_err(VmError::return_type)?;
    let memory = env.get_wasmer_memory(&store)?;
    let mut region = read_region(&memory, region_ptr)?;
    // don't forget to update region length
    region.length = data.len() as u32;

    if region.length > region.capacity {
        return Err(VmError::region_too_small(
            region.offset,
            region.capacity,
            BASE64.encode(data),
        ));
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
