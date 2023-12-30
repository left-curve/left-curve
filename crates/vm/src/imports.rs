use {
    crate::{Host, HostState},
    cw_sdk::Storage,
    data_encoding::BASE64,
    wasmi::Caller,
};

pub fn db_read<S>(caller: Caller<'_, HostState<S>>, key_ptr: u32) -> Result<u32, wasmi::Error>
where
    S: Storage,
{
    let mut host = Host::from(caller);
    let key = host.read_from_memory(key_ptr)?;

    // TODO: replace this with tracing debug
    println!("db_read: key = {}", BASE64.encode(&key));

    // read the value from host state
    // if doesn't exist, we return a zero pointer
    let Some(value) = host.data().kv.read(&key) else {
        return Ok(0);
    };

    // now we need to allocate a region in Wasm memory and put the value in
    host.write_to_memory(&value).map_err(Into::into)
}

pub fn db_write<S>(
    caller:    Caller<'_, HostState<S>>,
    key_ptr:   u32,
    value_ptr: u32,
) -> Result<(), wasmi::Error>
where
    S: Storage,
{
    let mut host = Host::from(caller);
    let key = host.read_from_memory(key_ptr)?;
    let value = host.read_from_memory(value_ptr)?;

    // TODO: replace this with tracing debug
    println!("db_write: key = {}, value = {}", BASE64.encode(&key), BASE64.encode(&value));

    host.data_mut().kv.write(&key, &value);

    Ok(())
}

pub fn db_remove<S>(caller: Caller<'_, HostState<S>>, key_ptr: u32) -> Result<(), wasmi::Error>
where
    S: Storage,
{
    let mut host = Host::from(caller);
    let key = host.read_from_memory(key_ptr)?;

    // TODO: replace this with tracing debug
    println!("db_remove: key = {}", BASE64.encode(&key));

    host.data_mut().kv.remove(&key);

    Ok(())
}

pub fn db_scan<S>(
    caller:  Caller<'_, HostState<S>>,
    min_ptr: u32,
    max_ptr: u32,
    order:   i32,
) -> Result<u32, wasmi::Error>
where
    S: Storage,
{
    let mut host = Host::from(caller);

    let min = if min_ptr != 0 {
        Some(host.read_from_memory(min_ptr)?)
    } else {
        None
    };
    let max = if max_ptr != 0 {
        Some(host.read_from_memory(max_ptr)?)
    } else {
        None
    };
    let order = order.try_into()?;

    // TODO: replace this with tracing debug
    println!(
        "db_scan: min = {:?}, max = {:?}, order = {order:?}",
        min.as_ref().map(|bytes| BASE64.encode(bytes)),
        max.as_ref().map(|bytes| BASE64.encode(bytes)),
    );

    // need to cast the bounds from Option<Vec<u8>> to Option<&[u8]>
    // this is a bit annoying
    let min = min.as_ref().map(|vec| vec.as_slice());
    let max = max.as_ref().map(|vec| vec.as_slice());

    Ok(host.data_mut().create_iterator(min, max, order))
}

pub fn db_next<S>(caller: Caller<'_, HostState<S>>, iterator_id: u32) -> Result<u32, wasmi::Error>
where
    S: Storage,
{
    let mut host = Host::from(caller);

    // TODO: replace this with tracing debug
    println!("db_next: iterator_id = {iterator_id}");

    // if the iterator has reached the end, we
    // - delete the iterator from host state;
    // - return a zero memory address to signal to the Wasm module that no data
    //   has been loaded into memory.
    let iter = host.data_mut().get_iterator_mut(iterator_id);
    let Some((key, value)) = iter.next() else {
        host.data_mut().drop_iterator(iterator_id);
        return Ok(0);
    };
    let data = encode_record(&key, &value);

    host.write_to_memory(&data).map_err(Into::into)
}

// pack a KV pair into a single byte array in the following format:
// key | value | len(key)
// where len() is two bytes (u16 big endian)
fn encode_record(k: &[u8], v: &[u8]) -> Vec<u8> {
    let mut data = Vec::with_capacity(k.len() + v.len() + 2);
    data.extend_from_slice(&k);
    data.extend_from_slice(&v);
    data.extend_from_slice(&(k.len() as u16).to_be_bytes());
    data
}
