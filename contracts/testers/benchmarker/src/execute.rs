use grug::{Api, MutableCtx, Number, Response, StdResult, Uint128};

use crate::{
    crypto::{blake3, secp256k1_verify, sha2_256},
    types::{CryptoApi, ExecuteTest},
};

pub(crate) fn do_test(ctx: MutableCtx, test: ExecuteTest) -> StdResult<Response> {
    match test {
        ExecuteTest::Math { iterations } => do_iteration(iterations),
        ExecuteTest::Crypto {
            on_host,
            crypto_api,
        } => do_crypto(ctx.api, on_host, crypto_api),
        ExecuteTest::DoNothingVecu8 { .. } | ExecuteTest::DoNothingBinary { .. } => {
            Ok(Response::default())
        },
    }
}

fn do_iteration(iterations: u64) -> StdResult<Response> {
    for _ in 0..iterations {
        // keep the same operation per iteration
        let number = Uint128::new(100);
        number.checked_add(number)?;
        number.checked_sub(number)?;
        number.checked_mul(number)?;
        number.checked_div(number)?;
        number.checked_pow(2)?;
    }
    Ok(Response::default())
}

fn do_crypto(api: &dyn Api, on_host: bool, crypto_api: CryptoApi) -> StdResult<Response> {
    match (on_host, crypto_api) {
        (true, CryptoApi::Sepc256k1verify { msg_hash, sig, pk }) => {
            api.secp256k1_verify(&msg_hash, &sig, &pk)?;
        },
        (false, CryptoApi::Sepc256k1verify { msg_hash, sig, pk }) => {
            secp256k1_verify(&msg_hash, &sig, &pk)?;
        },
        (true, CryptoApi::Sha2_256 { msg }) => {
            api.sha2_256(&msg);
        },
        (false, CryptoApi::Sha2_256 { msg }) => {
            sha2_256(&msg);
        },
        (true, CryptoApi::Blake3 { msg }) => {
            api.blake3(&msg);
        },
        (false, CryptoApi::Blake3 { msg }) => {
            blake3(&msg);
        },
    }
    Ok(Response::default())
}
