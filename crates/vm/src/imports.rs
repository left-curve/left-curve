use {
    crate::{read_from_memory, write_to_memory, Environment, Storage, VmResult},
    cw_std::Record,
    tracing::debug,
    wasmer::FunctionEnvMut,
};

pub fn db_read<S: Storage + 'static>(
    mut fe:  FunctionEnvMut<Environment<S>>,
    key_ptr: u32,
) -> VmResult<u32> {
    let (env, mut wasm_store) = fe.data_and_store_mut();

    let key = read_from_memory(env, &wasm_store, key_ptr)?;
    let maybe_value = env.with_store(|store| store.read(&key))?;
    // if doesn't exist, we return a zero pointer
    let Some(value) = maybe_value else {
        return Ok(0);
    };

    write_to_memory(env, &mut wasm_store, &value)
}

pub fn db_scan<S: Storage + 'static>(
    mut fe:  FunctionEnvMut<Environment<S>>,
    min_ptr: u32,
    max_ptr: u32,
    order:   i32,
) -> VmResult<i32>
{
    let (env, wasm_store) = fe.data_and_store_mut();

    let min = if min_ptr != 0 {
        Some(read_from_memory(env, &wasm_store, min_ptr)?)
    } else {
        None
    };
    let max = if max_ptr != 0 {
        Some(read_from_memory(env, &wasm_store, max_ptr)?)
    } else {
        None
    };
    // TODO: do not use unwrap here
    let order = order.try_into().unwrap();

    // need to cast the bounds from Option<Vec<u8>> to Option<&[u8]>. `as_deref` works!
    env.with_store_mut(|store| store.scan(min.as_deref(), max.as_deref(), order))
}

pub fn db_next<S: Storage + 'static>(
    mut fe:      FunctionEnvMut<Environment<S>>,
    iterator_id: i32,
) -> VmResult<u32>
{
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

    let (env, mut wasm_store) = fe.data_and_store_mut();

    let Some(record) = env.with_store_mut(|store| store.next(iterator_id))? else {
        // returning a zero memory address informs the Wasm module that the
        // iterator has reached its end, and no data is loaded into memory.
        return Ok(0);
    };

    write_to_memory(env, &mut wasm_store, &encode_record(record))
}

pub fn db_write<S: Storage + 'static>(
    mut fe:    FunctionEnvMut<Environment<S>>,
    key_ptr:   u32,
    value_ptr: u32,
) -> VmResult<()>
{
    let (env, wasm_store) = fe.data_and_store_mut();

    let key = read_from_memory(env, &wasm_store, key_ptr)?;
    let value = read_from_memory(env, &wasm_store, value_ptr)?;

    env.with_store_mut(|store| store.write(&key, &value))
}

pub fn db_remove<S: Storage + 'static>(mut fe: FunctionEnvMut<Environment<S>>, key_ptr: u32) -> VmResult<()> {
    let (env, wasm_store) = fe.data_and_store_mut();

    let key = read_from_memory(env, &wasm_store, key_ptr)?;

    env.with_store_mut(|store| store.remove(&key))
}

pub fn debug<S: 'static>(mut fe: FunctionEnvMut<Environment<S>>, msg_ptr: u32) -> VmResult<()> {
    let (env, wasm_store) = fe.data_and_store_mut();

    let msg_bytes = read_from_memory(env, &wasm_store, msg_ptr)?;
    let msg = String::from_utf8(msg_bytes)?;
    debug!(msg, "contract debug");

    Ok(())
}

pub fn secp256k1_verify<S: 'static>(
    mut fe: FunctionEnvMut<Environment<S>>,
    msg_hash_ptr: u32,
    sig_ptr:      u32,
    pk_ptr:       u32,
) -> VmResult<i32> {
    let (env, wasm_store) = fe.data_and_store_mut();

    let msg_hash = read_from_memory(env, &wasm_store, msg_hash_ptr)?;
    let sig = read_from_memory(env, &wasm_store, sig_ptr)?;
    let pk = read_from_memory(env, &wasm_store, pk_ptr)?;

    match cw_crypto::secp256k1_verify(&msg_hash, &sig, &pk) {
        Ok(()) => Ok(0),
        Err(_) => Ok(1),
    }
}

pub fn secp256r1_verify<S: 'static>(
    mut fe: FunctionEnvMut<Environment<S>>,
    msg_hash_ptr: u32,
    sig_ptr:      u32,
    pk_ptr:       u32,
) -> VmResult<i32> {
    let (env, wasm_store) = fe.data_and_store_mut();

    let msg_hash = read_from_memory(env, &wasm_store, msg_hash_ptr)?;
    let sig = read_from_memory(env, &wasm_store, sig_ptr)?;
    let pk = read_from_memory(env, &wasm_store, pk_ptr)?;

    match cw_crypto::secp256r1_verify(&msg_hash, &sig, &pk) {
        Ok(()) => Ok(0),
        Err(_) => Ok(1),
    }
}
