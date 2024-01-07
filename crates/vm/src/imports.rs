use {
    crate::{Host, Storage},
    anyhow::anyhow,
    cw_std::Record,
    tracing::debug,
    wasmi::Caller,
};

pub fn db_read<S: Storage>(
    caller:  Caller<'_, S>,
    key_ptr: u32,
) -> Result<u32, wasmi::Error>
{
    let mut host = Host::from(caller);

    let key = host.read_from_memory(key_ptr)?;
    // read the value from host state
    // if doesn't exist, we return a zero pointer
    let Some(value) = host.data().read(&key)? else {
        return Ok(0);
    };

    // now we need to allocate a region in Wasm memory and put the value in
    host.write_to_memory(&value).map_err(Into::into)
}

pub fn db_scan<S: Storage>(
    caller:  Caller<'_, S>,
    min_ptr: u32,
    max_ptr: u32,
    order:   i32,
) -> Result<i32, wasmi::Error>
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

    // need to cast the bounds from Option<Vec<u8>> to Option<&[u8]>
    // `as_deref` works!
    host.data_mut().scan(min.as_deref(), max.as_deref(), order).map_err(Into::into)
}

pub fn db_next<S: Storage>(
    caller:      Caller<'_, S>,
    iterator_id: i32,
) -> Result<u32, wasmi::Error>
{
    let mut host = Host::from(caller);

    let Some(record) = host.data_mut().next(iterator_id)? else {
        // returning a zero memory address informs the Wasm module that the
        // iterator has reached its end, and no data is loaded into memory.
        return Ok(0);
    };

    host.write_to_memory(&encode_record(record)).map_err(Into::into)
}

pub fn db_write<S: Storage>(
    caller:    Caller<'_, S>,
    key_ptr:   u32,
    value_ptr: u32,
) -> Result<(), wasmi::Error>
{
    let mut host = Host::from(caller);

    let key = host.read_from_memory(key_ptr)?;
    let value = host.read_from_memory(value_ptr)?;

    host.data_mut().write(&key, &value).map_err(Into::into)
}

pub fn db_remove<S: Storage>(
    caller:  Caller<'_, S>,
    key_ptr: u32,
) -> Result<(), wasmi::Error>
{
    let mut host = Host::from(caller);

    let key = host.read_from_memory(key_ptr)?;

    host.data_mut().remove(&key).map_err(Into::into)
}

// pack a KV pair into a single byte array in the following format:
// key | value | len(key)
// where len() is two bytes (u16 big endian)
#[inline]
fn encode_record((mut k, v): Record) -> Vec<u8> {
    let key_len = k.len();
    k.extend(v);
    k.extend_from_slice(&(key_len as u16).to_be_bytes());
    k
}

pub fn debug<S>(caller: Caller<'_, S>, msg_ptr: u32) -> Result<(), wasmi::Error> {
    let host = Host::from(caller);

    let msg_bytes = host.read_from_memory(msg_ptr)?;
    let msg = String::from_utf8(msg_bytes).map_err(|_| anyhow!("Invalid UTF8"))?;
    debug!(msg, "contract debug");

    Ok(())
}

pub fn secp256k1_verify<S>(
    caller:       Caller<'_, S>,
    msg_hash_ptr: u32,
    sig_ptr:      u32,
    pk_ptr:       u32,
) -> Result<i32, wasmi::Error> {
    let host = Host::from(caller);

    let msg_hash = host.read_from_memory(msg_hash_ptr)?;
    let sig = host.read_from_memory(sig_ptr)?;
    let pk = host.read_from_memory(pk_ptr)?;

    match cw_crypto::secp256k1_verify(&msg_hash, &sig, &pk) {
        Ok(()) => Ok(0),
        Err(_) => Ok(1),
    }
}

pub fn secp256r1_verify<S>(
    caller:       Caller<'_, S>,
    msg_hash_ptr: u32,
    sig_ptr:      u32,
    pk_ptr:       u32,
) -> Result<i32, wasmi::Error> {
    let host = Host::from(caller);

    let msg_hash = host.read_from_memory(msg_hash_ptr)?;
    let sig = host.read_from_memory(sig_ptr)?;
    let pk = host.read_from_memory(pk_ptr)?;

    match cw_crypto::secp256r1_verify(&msg_hash, &sig, &pk) {
        Ok(()) => Ok(0),
        Err(_) => Ok(1),
    }
}
