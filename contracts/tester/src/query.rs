use grug::{Binary, ByteArray, Empty, Hash256, Hash512, ImmutableCtx, Number, StdResult, Uint128};

pub fn query_loop(iterations: u64) -> StdResult<Empty> {
    // Keep the same operation per iteration for consistency
    for _ in 0..iterations {
        let number = Uint128::new(100);
        number.checked_add(number)?;
        number.checked_sub(number)?;
        number.checked_mul(number)?;
        number.checked_div(number)?;
        number.checked_pow(2)?;
    }

    Ok(Empty {})
}

pub fn query_force_write(_key: &str, _value: &str) -> Empty {
    #[cfg(target_arch = "wasm32")]
    {
        use grug::Region;

        extern "C" {
            fn db_write(key_ptr: usize, value_ptr: usize);
        }

        let key_region = Region::build(_key.as_bytes());
        let key_ptr = &*key_region as *const Region;

        let value_region = Region::build(_value.as_bytes());
        let value_ptr = &*value_region as *const Region;

        // This should fail!
        unsafe {
            db_write(key_ptr as usize, value_ptr as usize);
        }
    }

    Empty {}
}

pub fn query_verify_secp256k1(
    ctx: ImmutableCtx,
    pk: Binary,
    sig: ByteArray<64>,
    msg_hash: Hash256,
) -> StdResult<()> {
    ctx.api.secp256k1_verify(&msg_hash, &sig, &pk)
}

pub fn query_verify_secp256r1(
    ctx: ImmutableCtx,
    pk: Binary,
    sig: ByteArray<64>,
    msg_hash: Hash256,
) -> StdResult<()> {
    ctx.api.secp256r1_verify(&msg_hash, &sig, &pk)
}

pub fn query_verify_ed25519(
    ctx: ImmutableCtx,
    pk: ByteArray<32>,
    sig: ByteArray<64>,
    msg_hash: Hash512,
) -> StdResult<()> {
    ctx.api.ed25519_verify(&msg_hash, &sig, &pk)
}

pub fn query_recover_secp256k1(
    ctx: ImmutableCtx,
    sig: ByteArray<64>,
    msg_hash: Hash256,
    recovery_id: u8,
    compressed: bool,
) -> StdResult<Binary> {
    ctx.api
        .secp256k1_pubkey_recover(&msg_hash, &sig, recovery_id, compressed)
        .map(Into::into)
}

macro_rules! slice_of_slices {
    ($vec:expr) => {{
        let slice_of_slices: Vec<&[u8]> = $vec.iter().map(|v| &v[..]).collect();
        slice_of_slices
    }};
}

pub fn query_ed25519_batch_verify(
    ctx: ImmutableCtx,
    pks: Vec<ByteArray<32>>,
    sigs: Vec<ByteArray<64>>,
    prehash_msgs: Vec<Binary>,
) -> StdResult<()> {
    let m = slice_of_slices!(prehash_msgs);
    let s = slice_of_slices!(sigs);
    let p = slice_of_slices!(pks);

    ctx.api.ed25519_batch_verify(&m, &s, &p)
}
