use {
    crate::{BALANCES, METADATAS, NAMESPACE_OWNERS, SUPPLIES},
    anyhow::{bail, ensure},
    dango_types::bank::{ExecuteMsg, InstantiateMsg, Metadata},
    grug::{
        Addr, BankMsg, Denom, IsZero, MutableCtx, Number, NumberConst, Part, Response, StdResult,
        Storage, SudoCtx, Uint128,
    },
    std::collections::HashMap,
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    let mut supplies = HashMap::<Denom, Uint128>::new();

    for (address, coins) in msg.balances {
        for coin in coins {
            BALANCES.save(ctx.storage, (&address, &coin.denom), &coin.amount)?;

            match supplies.get_mut(&coin.denom) {
                Some(supply) => {
                    supply.checked_add_assign(coin.amount)?;
                },
                None => {
                    supplies.insert(coin.denom, coin.amount);
                },
            }
        }
    }

    for (denom, amount) in supplies {
        SUPPLIES.save(ctx.storage, &denom, &amount)?;
    }

    for (namespace, owner) in msg.namespaces {
        NAMESPACE_OWNERS.save(ctx.storage, &namespace, &owner)?;
    }

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::GrantNamespace { namespace, owner } => grant_namespace(ctx, namespace, owner),
        ExecuteMsg::SetMetadata { denom, metadata } => set_metadata(ctx, denom, metadata),
        ExecuteMsg::Mint { to, denom, amount } => mint(ctx, to, denom, amount),
        ExecuteMsg::Burn {
            from,
            denom,
            amount,
        } => burn(ctx, from, denom, amount),
        ExecuteMsg::ForceTransfer {
            from,
            to,
            denom,
            amount,
        } => force_transfer(ctx, from, to, denom, amount),
    }
}

fn grant_namespace(ctx: MutableCtx, namespace: Part, owner: Addr) -> anyhow::Result<Response> {
    let cfg = ctx.querier.query_config()?;

    // Only chain owner can grant namespace.
    ensure!(
        ctx.sender == cfg.owner,
        "you don't have the right, O you don't have the right"
    );

    NAMESPACE_OWNERS.may_update(ctx.storage, &namespace, |maybe_owner| {
        // TODO: for now, we don't support granting a namespace to multiple
        // owners or overwriting an existing owner.
        if let Some(existing_owner) = maybe_owner {
            bail!("namespace `{namespace}` already granted to `{existing_owner}`");
        }

        Ok(owner)
    })?;

    Ok(Response::new())
}

fn set_metadata(ctx: MutableCtx, denom: Denom, metadata: Metadata) -> anyhow::Result<Response> {
    ensure_namespace_owner(&ctx, &denom)?;

    METADATAS.save(ctx.storage, &denom, &metadata)?;

    Ok(Response::default())
}

fn mint(ctx: MutableCtx, to: Addr, denom: Denom, amount: Uint128) -> anyhow::Result<Response> {
    ensure_namespace_owner(&ctx, &denom)?;

    increase_supply(ctx.storage, &denom, amount)?;
    increase_balance(ctx.storage, &to, &denom, amount)?;

    Ok(Response::new())
}

fn burn(ctx: MutableCtx, from: Addr, denom: Denom, amount: Uint128) -> anyhow::Result<Response> {
    ensure_namespace_owner(&ctx, &denom)?;

    decrease_supply(ctx.storage, &denom, amount)?;
    decrease_balance(ctx.storage, &from, &denom, amount)?;

    Ok(Response::new())
}

fn ensure_namespace_owner(ctx: &MutableCtx, denom: &Denom) -> anyhow::Result<()> {
    match denom.namespace() {
        // The denom has a namespace. The namespace's owner can mint/burn.
        Some(part) => {
            let maybe_owner = NAMESPACE_OWNERS.may_load(ctx.storage, part)?;
            ensure!(
                maybe_owner == Some(ctx.sender),
                "sender does not own the namespace `{part}`"
            );
        },
        // The denom is a top-level denom (i.e. doesn't have a namespace).
        // Only the chain owner can mint/burn.
        None => {
            let cfg = ctx.querier.query_config()?;
            ensure!(
                ctx.sender == cfg.owner,
                "only chain owner can mint or burn top-level denoms"
            )
        },
    }

    Ok(())
}

fn force_transfer(
    ctx: MutableCtx,
    from: Addr,
    to: Addr,
    denom: Denom,
    amount: Uint128,
) -> anyhow::Result<Response> {
    let cfg = ctx.querier.query_config()?;

    // Only taxman can force transfer.
    ensure!(
        ctx.sender == cfg.taxman,
        "you don't have the right, O you don't have the right"
    );

    decrease_balance(ctx.storage, &from, &denom, amount)?;
    increase_balance(ctx.storage, &to, &denom, amount)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn bank_execute(ctx: SudoCtx, msg: BankMsg) -> StdResult<Response> {
    for coin in msg.coins {
        decrease_balance(ctx.storage, &msg.from, &coin.denom, coin.amount)?;
        increase_balance(ctx.storage, &msg.to, &coin.denom, coin.amount)?;
    }

    Ok(Response::new())
}

fn increase_supply(
    storage: &mut dyn Storage,
    denom: &Denom,
    amount: Uint128,
) -> StdResult<Option<Uint128>> {
    SUPPLIES.may_modify(storage, denom, |maybe_supply| {
        let supply = maybe_supply.unwrap_or(Uint128::ZERO).checked_add(amount)?;
        // Only write to storage if the supply is non-zero.
        if supply.is_zero() {
            Ok(None)
        } else {
            Ok(Some(supply))
        }
    })
}

fn decrease_supply(
    storage: &mut dyn Storage,
    denom: &Denom,
    amount: Uint128,
) -> StdResult<Option<Uint128>> {
    SUPPLIES.may_modify(storage, denom, |maybe_supply| {
        let supply = maybe_supply.unwrap_or(Uint128::ZERO).checked_sub(amount)?;
        // If supply is reduced to zero, delete it, to save disk space.
        if supply.is_zero() {
            Ok(None)
        } else {
            Ok(Some(supply))
        }
    })
}

fn increase_balance(
    storage: &mut dyn Storage,
    address: &Addr,
    denom: &Denom,
    amount: Uint128,
) -> StdResult<Option<Uint128>> {
    BALANCES.may_modify(storage, (address, denom), |maybe_balance| {
        let balance = maybe_balance.unwrap_or(Uint128::ZERO).checked_add(amount)?;
        // Only write to storage if the balance is non-zero.
        if balance.is_zero() {
            Ok(None)
        } else {
            Ok(Some(balance))
        }
    })
}

fn decrease_balance(
    storage: &mut dyn Storage,
    address: &Addr,
    denom: &Denom,
    amount: Uint128,
) -> StdResult<Option<Uint128>> {
    BALANCES.may_modify(storage, (address, denom), |maybe_balance| {
        let balance = maybe_balance.unwrap_or(Uint128::ZERO).checked_sub(amount)?;
        // If balance is reduced to zero, delete it, to save disk space.
        if balance.is_zero() {
            Ok(None)
        } else {
            Ok(Some(balance))
        }
    })
}
