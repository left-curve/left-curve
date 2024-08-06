use grug::{Empty, ImmutableCtx, Number, StdResult, Uint128};

use crate::CryptoVerifyType;

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

pub fn query_crypto_verify(
    ctx: ImmutableCtx,
    ty: CryptoVerifyType,
    pk: Vec<u8>,
    sig: Vec<u8>,
    msg_hash: Vec<u8>,
) -> StdResult<()> {
    match ty {
        CryptoVerifyType::Ed25519 => ctx.api.ed25519_verify(&msg_hash, &sig, &pk),
        CryptoVerifyType::Secp256k1 => ctx.api.secp256k1_verify(&msg_hash, &sig, &pk),
        CryptoVerifyType::Secp256r1 => ctx.api.secp256r1_verify(&msg_hash, &sig, &pk),
    }
}
