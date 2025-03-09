use {
    crate::{
        ExecuteMsg, InstantiateMsg, QueryMsg, execute_stack_overflow, force_write_on_query,
        infinite_loop, query_force_write, query_loop, query_recover_secp256k1,
        query_stack_overflow, query_verify_ed25519, query_verify_ed25519_batch,
        query_verify_secp256k1, query_verify_secp256r1,
    },
    grug::{ImmutableCtx, Json, JsonSerExt, MutableCtx, Response, StdResult},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(_ctx: MutableCtx, _msg: InstantiateMsg) -> StdResult<Response> {
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::InfiniteLoop {} => infinite_loop(),
        ExecuteMsg::ForceWriteOnQuery { key, value } => force_write_on_query(ctx, key, value),
        ExecuteMsg::StackOverflow {} => execute_stack_overflow(ctx),
    }
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::Loop { iterations } => query_loop(iterations)?.to_json_value(),
        QueryMsg::ForceWrite { key, value } => {
            query_force_write(&key, &value);
            ().to_json_value()
        },
        QueryMsg::StackOverflow {} => query_stack_overflow(ctx)?.to_json_value(),
        QueryMsg::VerifySecp256r1 { pk, sig, msg_hash } => {
            query_verify_secp256r1(ctx, pk, sig, msg_hash)?.to_json_value()
        },
        QueryMsg::VerifySecp256k1 { pk, sig, msg_hash } => {
            query_verify_secp256k1(ctx, pk, sig, msg_hash)?.to_json_value()
        },
        QueryMsg::RecoverSecp256k1 {
            sig,
            msg_hash,
            recovery_id,
            compressed,
        } => query_recover_secp256k1(ctx, sig, msg_hash, recovery_id, compressed)?.to_json_value(),
        QueryMsg::VerifyEd25519 { pk, sig, msg_hash } => {
            query_verify_ed25519(ctx, pk, sig, msg_hash)?.to_json_value()
        },
        QueryMsg::VerifyEd25519Batch {
            pks,
            sigs,
            prehash_msgs,
        } => query_verify_ed25519_batch(ctx, pks, sigs, prehash_msgs)?.to_json_value(),
    }
}
