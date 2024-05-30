use {
    crate::{Environment, Region, VmError, VmResult},
    data_encoding::BASE64,
    wasmer::{AsStoreMut, AsStoreRef, MemoryView, WasmPtr},
};

pub fn read_from_memory(
    env: &mut Environment,
    wasm_store: &impl AsStoreRef,
    region_ptr: u32,
) -> VmResult<Vec<u8>> {
    let memory = env.memory(&wasm_store)?;

    // read region
    let region = read_region(&memory, region_ptr)?;

    // read memory area indicated by region
    let mut buf = vec![0u8; region.length as usize];
    memory.read(region.offset as u64, &mut buf)?;

    Ok(buf)
}

pub fn read_then_wipe(
    env: &mut Environment,
    wasm_store: &mut impl AsStoreMut,
    region_ptr: u32,
) -> VmResult<Vec<u8>> {
    let data = read_from_memory(env, wasm_store, region_ptr)?;
    env.call_function0(wasm_store, "deallocate", &[region_ptr.into()])?;
    Ok(data)
}

pub fn write_to_memory(
    env: &mut Environment,
    wasm_store: &mut impl AsStoreMut,
    data: &[u8],
) -> VmResult<u32> {
    // call the `allocate` export to reserve an area in Wasm memory
    let region_ptr: u32 = env
        .call_function1(wasm_store, "allocate", &[(data.len() as u32).into()])?
        .try_into()
        .map_err(VmError::ReturnType)?;
    let memory = env.memory(&wasm_store)?;
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
    wptr.deref(memory).read().map_err(Into::into)
    // TODO: do some sanity checks on the Region?
}

fn write_region(memory: &MemoryView, offset: u32, region: Region) -> VmResult<()> {
    let wptr = <WasmPtr<Region>>::new(offset);
    wptr.deref(memory).write(region).map_err(Into::into)
}
