use {crate::Host, cw_sdk::Storage, wasmi::Caller};

pub fn db_read<T>(caller: Caller<'_, T>, key_ptr: u32) -> Result<u32, wasmi::Error>
where
    T: Storage,
{
    let mut host = Host::from(caller);
    let key = host.read_from_memory(key_ptr)?;

    // read the value from host state
    // if doesn't exist, we return a zero pointer
    let Some(value) = host.data().read(&key) else {
        return Ok(0);
    };

    // now we need to allocate a region in Wasm memory and put the value in
    let value_ptr = host.write_to_memory(&value)?;

    Ok(value_ptr)
}

pub fn db_write<T>(caller: Caller<'_, T>, key_ptr: u32, value_ptr: u32) -> Result<(), wasmi::Error>
where
    T: Storage,
{
    let mut host = Host::from(caller);
    let key = host.read_from_memory(key_ptr)?;
    let value = host.read_from_memory(value_ptr)?;

    host.data_mut().write(&key, &value);

    Ok(())
}

pub fn db_remove<T>(caller: Caller<'_, T>, key_ptr: u32) -> Result<(), wasmi::Error>
where
    T: Storage,
{
    let mut host = Host::from(caller);
    let key = host.read_from_memory(key_ptr)?;

    host.data_mut().remove(&key);

    Ok(())
}
