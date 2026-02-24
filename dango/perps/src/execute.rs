mod deposit;
mod submit_order;
mod withdraw;

use {
    crate::{PAIR_PARAMS, PAIR_STATES, PARAM, STATE},
    dango_types::{
        UsdValue,
        perps::{ExecuteMsg, InstantiateMsg, PairState, State},
    },
    grug::{Addr, MutableCtx, Response, Uint128, addr},
};

/// Virtual shares added to total supply in share price calculations.
/// Prevents the first-depositor attack (ERC-4626 inflation attack) by
/// ensuring the share price cannot be trivially inflated.
const VIRTUAL_SHARES: Uint128 = Uint128::new(1_000_000);

/// Virtual assets added to vault equity in share price calculations.
/// Works in tandem with `VIRTUAL_SHARES` to set the initial share price
/// and prevent share inflation attacks.
const VIRTUAL_ASSETS: UsdValue = UsdValue::new_int(1);

/// Address of the bank contract.
const BANK: Addr = addr!("e0b49f70991ecab05d5d7dc1f71e4ede63c8f2b7");

/// Address of the oracle contract.
const ORACLE: Addr = addr!("cedc5f73cbb963a48471b849c3650e6e34cd3b6d");

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    PARAM.save(ctx.storage, &msg.param)?;
    STATE.save(ctx.storage, &State::default())?;

    for (pair_id, pair_param) in msg.pair_params {
        PAIR_PARAMS.save(ctx.storage, &pair_id, &pair_param)?;
        PAIR_STATES.save(ctx.storage, &pair_id, &PairState::new(ctx.block.timestamp))?;
    }

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::Deposit { min_shares_to_mint } => deposit::deposit(ctx, min_shares_to_mint),
        ExecuteMsg::Unlock {} => withdraw::withdraw(ctx),
        ExecuteMsg::SubmitOrder {
            pair_id,
            size,
            kind,
            reduce_only,
        } => submit_order::submit_order(ctx, pair_id, size, kind, reduce_only),
        _ => todo!(),
    }
}
