use {
    crate::{Host, HostState},
    data_encoding::BASE64,
    wasmi::Caller,
};

pub fn db_read<S>(caller: Caller<'_, S>, key_ptr: u32) -> Result<u32, wasmi::Error>
where
    S: HostState,
{
    let mut host = Host::from(caller);
    let key = host.read_from_memory(key_ptr)?;

    // TODO: replace this with tracing debug
    println!("db_read: key = {}", BASE64.encode(&key));

    // read the value from host state
    // if doesn't exist, we return a zero pointer
    let Some(value) = host.data().read(&key)? else {
        return Ok(0);
    };

    // now we need to allocate a region in Wasm memory and put the value in
    host.write_to_memory(&value).map_err(Into::into)
}

pub fn db_write<S>(caller: Caller<'_, S>, key_ptr: u32, value_ptr: u32) -> Result<(), wasmi::Error>
where
    S: HostState,
{
    let mut host = Host::from(caller);
    let key = host.read_from_memory(key_ptr)?;
    let value = host.read_from_memory(value_ptr)?;

    // TODO: replace this with tracing debug
    println!("db_write: key = {}, value = {}", BASE64.encode(&key), BASE64.encode(&value));

    host.data_mut().write(&key, &value).map_err(Into::into)
}

pub fn db_remove<S>(caller: Caller<'_, S>, key_ptr: u32) -> Result<(), wasmi::Error>
where
    S: HostState,
{
    let mut host = Host::from(caller);
    let key = host.read_from_memory(key_ptr)?;

    // TODO: replace this with tracing debug
    println!("db_remove: key = {}", BASE64.encode(&key));

    host.data_mut().remove(&key).map_err(Into::into)
}

pub fn db_scan<S>(
    caller:  Caller<'_, S>,
    min_ptr: u32,
    max_ptr: u32,
    order:   i32,
) -> Result<u32, wasmi::Error>
where
    S: HostState,
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

    host.data_mut().scan(min, max, order).map_err(Into::into)
}

pub fn db_next<S>(caller: Caller<'_, S>, iterator_id: u32) -> Result<u32, wasmi::Error>
where
    S: HostState,
{
    let mut host = Host::from(caller);

    // TODO: replace this with tracing debug
    println!("db_next: iterator_id = {iterator_id}");

    let Some(record) = host.data_mut().next(iterator_id)? else {
        // returning a zero memory address informs the Wasm module that the
        // iterator has reached its end, and no data is loaded into memory.
        return Ok(0);
    };

    host.write_to_memory(&encode_record(record)).map_err(Into::into)
}

// pack a KV pair into a single byte array in the following format:
// key | value | len(key)
// where len() is two bytes (u16 big endian)
fn encode_record((mut k, v): (Vec<u8>, Vec<u8>)) -> Vec<u8> {
    let key_len = k.len();
    k.extend(v);
    k.extend_from_slice(&(key_len as u16).to_be_bytes());
    k
}
