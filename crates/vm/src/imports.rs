use {
    crate::{read_from_memory, write_to_memory, Environment, VmResult},
    cw_std::{from_json_slice, to_json_vec, Addr, QueryRequest, Record},
    tracing::info,
    wasmer::FunctionEnvMut,
};

pub fn db_read(mut fe: FunctionEnvMut<Environment>, key_ptr: u32) -> VmResult<u32> {
    let (env, mut wasm_store) = fe.data_and_store_mut();

    let key = read_from_memory(env, &wasm_store, key_ptr)?;
    let maybe_value = env.with_context_data(|ctx| ctx.store.read(&key))?;
    // if doesn't exist, we return a zero pointer
    let Some(value) = maybe_value else {
        return Ok(0);
    };

    write_to_memory(env, &mut wasm_store, &value)
}

pub fn db_scan(
    mut fe: FunctionEnvMut<Environment>,
    min_ptr: u32,
    max_ptr: u32,
    order: i32,
) -> VmResult<i32> {
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
    env.with_context_data_mut(|ctx| ctx.store.scan(min.as_deref(), max.as_deref(), order))
}

pub fn db_next(mut fe: FunctionEnvMut<Environment>, iterator_id: i32) -> VmResult<u32> {
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

    let Some(record) = env.with_context_data_mut(|ctx| ctx.store.next(iterator_id))? else {
        // returning a zero memory address informs the Wasm module that the
        // iterator has reached its end, and no data is loaded into memory.
        return Ok(0);
    };

    write_to_memory(env, &mut wasm_store, &encode_record(record))
}

pub fn db_write(mut fe: FunctionEnvMut<Environment>, key_ptr: u32, value_ptr: u32) -> VmResult<()> {
    let (env, wasm_store) = fe.data_and_store_mut();

    let key = read_from_memory(env, &wasm_store, key_ptr)?;
    let value = read_from_memory(env, &wasm_store, value_ptr)?;

    env.with_context_data_mut(|ctx| ctx.store.write(&key, &value))
}

pub fn db_remove(mut fe: FunctionEnvMut<Environment>, key_ptr: u32) -> VmResult<()> {
    let (env, wasm_store) = fe.data_and_store_mut();

    let key = read_from_memory(env, &wasm_store, key_ptr)?;

    env.with_context_data_mut(|ctx| ctx.store.remove(&key))
}

pub fn debug(mut fe: FunctionEnvMut<Environment>, addr_ptr: u32, msg_ptr: u32) -> VmResult<()> {
    let (env, wasm_store) = fe.data_and_store_mut();

    let addr_bytes = read_from_memory(env, &wasm_store, addr_ptr)?;
    let addr = Addr::try_from(addr_bytes)?;
    let msg_bytes = read_from_memory(env, &wasm_store, msg_ptr)?;
    let msg = String::from_utf8(msg_bytes)?;

    info!(contract = addr.to_string(), msg, "Contract emitted debug message");

    Ok(())
}

pub fn query_chain(mut fe: FunctionEnvMut<Environment>, req_ptr: u32) -> VmResult<u32> {
    let (env, mut wasm_store) = fe.data_and_store_mut();

    let req_bytes = read_from_memory(env, &wasm_store, req_ptr)?;
    let req: QueryRequest = from_json_slice(req_bytes)?;

    let res = env.with_context_data(|ctx| ctx.querier.query_chain(req))?;
    let res_bytes = to_json_vec(&res)?;

    write_to_memory(env, &mut wasm_store, &res_bytes)
}

pub fn secp256k1_verify(
    mut fe: FunctionEnvMut<Environment>,
    msg_hash_ptr: u32,
    sig_ptr: u32,
    pk_ptr: u32,
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

pub fn secp256r1_verify(
    mut fe: FunctionEnvMut<Environment>,
    msg_hash_ptr: u32,
    sig_ptr: u32,
    pk_ptr: u32,
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
