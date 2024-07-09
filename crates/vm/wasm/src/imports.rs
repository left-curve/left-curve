use {
    crate::{
        read_from_memory, write_to_memory, Environment, Iterator, VmError, VmResult, GAS_COSTS,
    },
    grug_types::{
        decode_sections, from_json_slice, to_json_vec, Addr, Querier, QueryRequest, Record, Storage,
    },
    tracing::info,
    wasmer::FunctionEnvMut,
};

pub fn db_read(mut fe: FunctionEnvMut<Environment>, key_ptr: u32) -> VmResult<u32> {
    let (env, mut store) = fe.data_and_store_mut();

    let key = read_from_memory(env, &store, key_ptr)?;

    // If the record doesn't exist, return a zero pointer.
    let Some(value) = env.storage.read(&key) else {
        return Ok(0);
    };

    write_to_memory(env, &mut store, &value)
}

pub fn db_scan(
    mut fe: FunctionEnvMut<Environment>,
    min_ptr: u32,
    max_ptr: u32,
    order: i32,
) -> VmResult<i32> {
    let (env, store) = fe.data_and_store_mut();

    // Parse iteration parameters provided by the module and create iterator.
    let min = if min_ptr != 0 {
        Some(read_from_memory(env, &store, min_ptr)?)
    } else {
        None
    };
    let max = if max_ptr != 0 {
        Some(read_from_memory(env, &store, max_ptr)?)
    } else {
        None
    };
    let order = order.try_into()?;
    let iterator = Iterator::new(min, max, order);

    Ok(env.add_iterator(iterator))
}

pub fn db_next(mut fe: FunctionEnvMut<Environment>, iterator_id: i32) -> VmResult<u32> {
    let (env, mut store) = fe.data_and_store_mut();

    // If the iterator has reached its end, return a zero pointer.
    let Some(record) = env.advance_iterator(iterator_id)? else {
        return Ok(0);
    };

    write_to_memory(env, &mut store, &encode_record(record))
}

pub fn db_next_key(mut fe: FunctionEnvMut<Environment>, iterator_id: i32) -> VmResult<u32> {
    let (env, mut store) = fe.data_and_store_mut();

    // If the iterator has reached its end, return a zero pointer.
    let Some((key, _)) = env.advance_iterator(iterator_id)? else {
        return Ok(0);
    };

    write_to_memory(env, &mut store, &key)
}

pub fn db_next_value(mut fe: FunctionEnvMut<Environment>, iterator_id: i32) -> VmResult<u32> {
    let (env, mut store) = fe.data_and_store_mut();

    // If the iterator has reached its end, return a zero pointer.
    let Some((_, value)) = env.advance_iterator(iterator_id)? else {
        return Ok(0);
    };

    write_to_memory(env, &mut store, &value)
}

pub fn db_write(mut fe: FunctionEnvMut<Environment>, key_ptr: u32, value_ptr: u32) -> VmResult<()> {
    let (env, store) = fe.data_and_store_mut();

    // Make sure the storage isn't set to be read only.
    //
    // This is the case for the `query`, `bank_query`, and `ibc_client_query`
    // calls. During these calls, the contract isn't allowed to call the imports
    // that mutates the state, namely: `db_write`, `db_remove`, and `db_remove_range`.
    if env.storage_readonly {
        return Err(VmError::ReadOnly);
    }

    let key = read_from_memory(env, &store, key_ptr)?;
    let value = read_from_memory(env, &store, value_ptr)?;

    env.storage.write(&key, &value);

    // Delete all existing iterators. This is necessary if the storage is to be
    // mutated.
    //
    // Let's consider what happens if we fail to do this.
    //
    // Assume the storage has the following keys: `a`, `b`, `c`. An existing
    // iterator with ascending order is now at `b`. If we are to call `db_next`
    // now, it would return the `c` record.
    //
    // Now, we perfrom `db_write` to insert a new record with key `bb`. Now the
    // storage contains: `a`, `b`, `bb`, `c`.
    //
    // Now we call `db_next`. It will still return `b`. This is an incorrect
    // result: should be `bb` instead!
    //
    // Think about this the other way: having an active iterator is like holding
    // an immutable reference to the storage (though there isn't actually a ref
    // since we're working over the FFI). Performing a `db_write` requires a
    // mutable reference, which requires the immutable ref to be dropped first,
    // which involves deleting the iterator.
    env.clear_iterators();

    Ok(())
}

pub fn db_remove(mut fe: FunctionEnvMut<Environment>, key_ptr: u32) -> VmResult<()> {
    let (env, store) = fe.data_and_store_mut();

    if env.storage_readonly {
        return Err(VmError::ReadOnly);
    }

    let key = read_from_memory(env, &store, key_ptr)?;

    env.storage.remove(&key);

    // Delete all existing iterators; same reasoning as in `db_write`.
    env.clear_iterators();

    Ok(())
}

pub fn db_remove_range(
    mut fe: FunctionEnvMut<Environment>,
    min_ptr: u32,
    max_ptr: u32,
) -> VmResult<()> {
    let (env, store) = fe.data_and_store_mut();

    if env.storage_readonly {
        return Err(VmError::ReadOnly);
    }

    let min = if min_ptr != 0 {
        Some(read_from_memory(env, &store, min_ptr)?)
    } else {
        None
    };
    let max = if max_ptr != 0 {
        Some(read_from_memory(env, &store, max_ptr)?)
    } else {
        None
    };

    env.storage.remove_range(min.as_deref(), max.as_deref());

    // Delete all existing iterators; same reasoning as in `db_write`.
    env.clear_iterators();

    Ok(())
}

pub fn debug(mut fe: FunctionEnvMut<Environment>, addr_ptr: u32, msg_ptr: u32) -> VmResult<()> {
    let (env, store) = fe.data_and_store_mut();

    let addr_bytes = read_from_memory(env, &store, addr_ptr)?;
    let addr = Addr::try_from(addr_bytes)?;
    let msg_bytes = read_from_memory(env, &store, msg_ptr)?;
    let msg = String::from_utf8(msg_bytes)?;

    info!(
        contract = addr.to_string(),
        msg, "Contract emitted debug message"
    );

    Ok(())
}

pub fn query_chain(mut fe: FunctionEnvMut<Environment>, req_ptr: u32) -> VmResult<u32> {
    let (env, mut store) = fe.data_and_store_mut();

    let req_bytes = read_from_memory(env, &store, req_ptr)?;
    let req: QueryRequest = from_json_slice(req_bytes)?;

    let res = env.querier.query_chain(req)?;
    let res_bytes = to_json_vec(&res)?;

    write_to_memory(env, &mut store, &res_bytes)
}

pub fn secp256k1_verify(
    mut fe: FunctionEnvMut<Environment>,
    msg_hash_ptr: u32,
    sig_ptr: u32,
    pk_ptr: u32,
) -> VmResult<i32> {
    let (env, mut store) = fe.data_and_store_mut();

    let msg_hash = read_from_memory(env, &store, msg_hash_ptr)?;
    let sig = read_from_memory(env, &store, sig_ptr)?;
    let pk = read_from_memory(env, &store, pk_ptr)?;

    env.consume_external_gas(
        &mut store,
        GAS_COSTS.secp256k1_verify_cost,
        "secp256k1_verify_cost",
    )?;

    match grug_crypto::secp256k1_verify(&msg_hash, &sig, &pk) {
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
    let (env, mut store) = fe.data_and_store_mut();

    let msg_hash = read_from_memory(env, &store, msg_hash_ptr)?;
    let sig = read_from_memory(env, &store, sig_ptr)?;
    let pk = read_from_memory(env, &store, pk_ptr)?;

    env.consume_external_gas(
        &mut store,
        GAS_COSTS.secp256k1_verify_cost,
        "secp256r1_verify_cost",
    )?;

    match grug_crypto::secp256r1_verify(&msg_hash, &sig, &pk) {
        Ok(()) => Ok(0),
        Err(_) => Ok(1),
    }
}

pub fn secp256k1_pubkey_recover(
    mut fe: FunctionEnvMut<Environment>,
    msg_hash_ptr: u32,
    sig_ptr: u32,
    recovery_id: u8,
    compressed: u8,
) -> VmResult<u32> {
    let (env, mut store) = fe.data_and_store_mut();

    let msg_hash = read_from_memory(env, &store, msg_hash_ptr)?;
    let sig = read_from_memory(env, &store, sig_ptr)?;

    let compressed = match compressed {
        0 => false,
        1 => true,
        _ => return Ok(0),
    };

    env.consume_external_gas(
        &mut store,
        GAS_COSTS.secp256k1_pubkey_recover,
        "secp256k1_pubkey_recover",
    )?;

    match grug_crypto::secp256k1_pubkey_recover(&msg_hash, &sig, recovery_id, compressed) {
        Ok(pk) => write_to_memory(env, &mut store, &pk),
        Err(_) => Ok(0),
    }
}

pub fn ed25519_verify(
    mut fe: FunctionEnvMut<Environment>,
    msg_hash_ptr: u32,
    sig_ptr: u32,
    pk_ptr: u32,
) -> VmResult<i32> {
    let (env, mut store) = fe.data_and_store_mut();

    let msg_hash = read_from_memory(env, &store, msg_hash_ptr)?;
    let sig = read_from_memory(env, &store, sig_ptr)?;
    let pk = read_from_memory(env, &store, pk_ptr)?;

    env.consume_external_gas(
        &mut store,
        GAS_COSTS.ed25519_verify_cost,
        "secp256k1_pubkey_recover",
    )?;

    match grug_crypto::ed25519_verify(&msg_hash, &sig, &pk) {
        Ok(()) => Ok(0),
        Err(_) => Ok(1),
    }
}

pub fn ed25519_batch_verify(
    mut fe: FunctionEnvMut<Environment>,
    msgs_hash_ptr: u32,
    sigs_ptr: u32,
    pks_ptr: u32,
) -> VmResult<i32> {
    let (env, mut store) = fe.data_and_store_mut();

    let msgs_hash = read_from_memory(env, &store, msgs_hash_ptr)?;
    let sigs = read_from_memory(env, &store, sigs_ptr)?;
    let pks = read_from_memory(env, &store, pks_ptr)?;

    let msgs_hash = decode_sections(&msgs_hash);
    let sigs = decode_sections(&sigs);
    let pks = decode_sections(&pks);

    env.consume_external_gas(
        &mut store,
        GAS_COSTS.ed25519_batch_verify_cost.cost(msgs_hash.len()),
        "ed25519_batch_verify",
    )?;

    match grug_crypto::ed25519_batch_verify(&msgs_hash, &sigs, &pks) {
        Ok(()) => Ok(0),
        Err(_) => Ok(1),
    }
}

macro_rules! impl_hash_method {
    ($name:ident) => {
        pub fn $name(mut fe: FunctionEnvMut<Environment>, data_ptr: u32) -> VmResult<u32> {
            let (env, mut store) = fe.data_and_store_mut();

            let data = read_from_memory(env, &store, data_ptr)?;

            env.consume_external_gas(
                &mut store,
                GAS_COSTS.$name.cost(data.len()),
                "ed25519_batch_verify",
            )?;

            let hash = grug_crypto::$name(&data);

            write_to_memory(env, &mut store, &hash)
        }
    };
}

impl_hash_method!(sha2_256);
impl_hash_method!(sha2_512);
impl_hash_method!(sha2_512_truncated);
impl_hash_method!(sha3_256);
impl_hash_method!(sha3_512);
impl_hash_method!(sha3_512_truncated);
impl_hash_method!(keccak256);
impl_hash_method!(blake2s_256);
impl_hash_method!(blake2b_512);
impl_hash_method!(blake3);

/// Pack a KV pair into a single byte array in the following format:
///
/// ```plain
/// key | value | len(key)
/// ```
///
/// where `len()` is two bytes (u16 big endian).
#[inline]
fn encode_record((mut k, v): Record) -> Vec<u8> {
    let key_len = k.len();
    k.extend(v);
    k.extend_from_slice(&(key_len as u16).to_be_bytes());
    k
}
