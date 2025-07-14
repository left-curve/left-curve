use {
    crate::QueryStackOverflowRequest,
    grug::{Binary, ImmutableCtx, Number, QuerierExt, StdResult, Uint128, UnaryNumber},
};

pub fn query_loop(iterations: u64) -> StdResult<()> {
    // Keep the same operation per iteration for consistency
    for _ in 0..iterations {
        let number = Uint128::new(100);
        number.checked_add(number)?;
        number.checked_sub(number)?;
        number.checked_mul(number)?;
        number.checked_div(number)?;
        number.checked_pow(2)?;
    }

    Ok(())
}

pub fn query_force_write(_key: &str, _value: &str) {
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
}

pub fn query_verify_secp256r1(
    ctx: ImmutableCtx,
    pk: Binary,
    sig: Binary,
    msg_hash: Binary,
) -> StdResult<()> {
    ctx.api.secp256r1_verify(&msg_hash, &sig, &pk)
}

pub fn query_verify_secp256k1(
    ctx: ImmutableCtx,
    pk: Binary,
    sig: Binary,
    msg_hash: Binary,
) -> StdResult<()> {
    ctx.api.secp256k1_verify(&msg_hash, &sig, &pk)
}

pub fn query_recover_secp256k1(
    ctx: ImmutableCtx,
    sig: Binary,
    msg_hash: Binary,
    recovery_id: u8,
    compressed: bool,
) -> StdResult<Binary> {
    ctx.api
        .secp256k1_pubkey_recover(&msg_hash, &sig, recovery_id, compressed)
        .map(Into::into)
}

pub fn query_verify_ed25519(
    ctx: ImmutableCtx,
    pk: Binary,
    sig: Binary,
    msg_hash: Binary,
) -> StdResult<()> {
    ctx.api.ed25519_verify(&msg_hash, &sig, &pk)
}

macro_rules! slice_of_slices {
    ($vec:expr) => {{
        $vec.iter().map(|v| &v[..]).collect::<Vec<_>>()
    }};
}

pub fn query_verify_ed25519_batch(
    ctx: ImmutableCtx,
    pks: Vec<Binary>,
    sigs: Vec<Binary>,
    prehash_msgs: Vec<Binary>,
) -> StdResult<()> {
    let m = slice_of_slices!(prehash_msgs);
    let s = slice_of_slices!(sigs);
    let p = slice_of_slices!(pks);

    ctx.api.ed25519_batch_verify(&m, &s, &p)
}

pub fn query_stack_overflow(ctx: ImmutableCtx) -> StdResult<()> {
    ctx.querier
        .query_wasm_smart(ctx.contract, QueryStackOverflowRequest {})?;

    Ok(())
}
