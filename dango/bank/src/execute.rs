use {
    crate::{BALANCES, METADATAS, NAMESPACE_OWNERS, ORPHANED_TRANSFERS, SUPPLIES},
    anyhow::{anyhow, bail, ensure},
    dango_types::bank::{
        Burned, ExecuteMsg, InstantiateMsg, Metadata, Minted, Received, Sent, TransferOrphaned,
    },
    grug::{
        Addr, BankMsg, Coins, Denom, EventBuilder, IsZero, MutableCtx, Number, NumberConst, Part,
        QuerierExt, Response, StdError, StdResult, Storage, SudoCtx, Uint128,
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

    for (denom, metadata) in msg.metadatas {
        METADATAS.save(ctx.storage, &denom, &metadata)?;
    }

    Ok(Response::new()) // No need to emit events during genesis.
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    ensure!(ctx.funds.is_empty(), "don't send funds to bank contract");

    match msg {
        ExecuteMsg::SetNamespaceOwner { namespace, owner } => {
            set_namespace_owner(ctx, namespace, owner)
        },
        ExecuteMsg::SetMetadata { denom, metadata } => set_metadata(ctx, denom, metadata),
        ExecuteMsg::Mint { to, coins } => mint(ctx, to, coins),
        ExecuteMsg::Burn { from, coins } => burn(ctx, from, coins),
        ExecuteMsg::ForceTransfer { from, to, coins } => force_transfer(ctx, from, to, coins),
        ExecuteMsg::RecoverTransfer { sender, recipient } => {
            recover_transfer(ctx, sender, recipient)
        },
    }
}

fn set_namespace_owner(ctx: MutableCtx, namespace: Part, owner: Addr) -> anyhow::Result<Response> {
    // Only chain owner can grant namespace.
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
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

fn mint(ctx: MutableCtx, to: Addr, coins: Coins) -> anyhow::Result<Response> {
    // Handle orphaned transfers.
    // See the comments in `bank_execute` for more details.
    let recipient_exists = ctx.querier.query_contract(to).is_ok();
    let recipient = if recipient_exists {
        to
    } else {
        ORPHANED_TRANSFERS.may_update(ctx.storage, (ctx.sender, to), |transfers| {
            let mut transfers = transfers.unwrap_or_default();
            transfers.insert_many(coins.clone())?;
            Ok::<_, StdError>(transfers)
        })?;

        ctx.contract
    };

    for coin in &coins {
        ensure_namespace_owner(&ctx, coin.denom)?;

        increase_supply(ctx.storage, coin.denom, *coin.amount)?;
        increase_balance(ctx.storage, &recipient, coin.denom, *coin.amount)?;
    }

    Ok(Response::new()
        .add_event(Minted {
            user: recipient,
            minter: ctx.sender,
            coins: coins.clone(),
        })?
        .may_add_event(if !recipient_exists {
            Some(TransferOrphaned {
                from: ctx.sender,
                to: recipient,
                coins,
            })
        } else {
            None
        })?)
}

fn burn(ctx: MutableCtx, from: Addr, coins: Coins) -> anyhow::Result<Response> {
    for coin in &coins {
        ensure_namespace_owner(&ctx, coin.denom)?;

        decrease_supply(ctx.storage, coin.denom, *coin.amount)?;
        decrease_balance(ctx.storage, &from, coin.denom, *coin.amount)?;
    }

    Ok(Response::new().add_event(Burned {
        user: from,
        burner: ctx.sender,
        coins,
    })?)
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
            ensure!(
                ctx.sender == ctx.querier.query_owner()?,
                "only chain owner can mint or burn top-level denoms"
            );
        },
    }

    Ok(())
}

/// Note: we don't handle orphaned transfers here. This function can only be
/// called by the taxman contract. We assume the taxman is properly programmed
/// to never force an orphaned transfer.
fn force_transfer(ctx: MutableCtx, from: Addr, to: Addr, coins: Coins) -> anyhow::Result<Response> {
    // Only taxman can force transfer.
    ensure!(
        ctx.sender == ctx.querier.query_taxman()?,
        "you don't have the right, O you don't have the right"
    );

    for coin in &coins {
        decrease_balance(ctx.storage, &from, coin.denom, *coin.amount)?;
        increase_balance(ctx.storage, &to, coin.denom, *coin.amount)?;
    }

    Ok(Response::new()
        .add_event(Sent {
            user: from,
            to,
            coins: coins.clone(),
        })?
        .add_event(Received {
            user: to,
            from,
            coins,
        })?)
}

fn recover_transfer(ctx: MutableCtx, sender: Addr, recipient: Addr) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == sender || ctx.sender == recipient,
        "only the sender or the recipient can recover an orphaned transfer"
    );

    let Some(coins) = ORPHANED_TRANSFERS.may_take(ctx.storage, (sender, recipient))? else {
        // Orphaned transfer not found.
        // Do nothing (return with no-op; do not throw error).
        return Ok(Response::new());
    };

    for coin in &coins {
        decrease_balance(ctx.storage, &ctx.contract, coin.denom, *coin.amount)?;
        increase_balance(ctx.storage, &ctx.sender, coin.denom, *coin.amount)?;
    }

    Ok(Response::new()
        .add_event(Sent {
            user: ctx.contract,
            to: ctx.sender,
            coins: coins.clone(),
        })?
        .add_event(Received {
            user: ctx.sender,
            from: ctx.contract,
            coins,
        })?)
}

/// There are two major problems with existing blockchain systems related to
/// token transfers:
///
/// 1. **It's not possible for the recipient to reject a token transfer.**
///
///    For example, a user who wants to unwrap their Wrapped Ether (WETH) tokens
///    may mistakenly send the tokens to the WETH contract, while the correct
///    way of doing it is to call the `withdraw` method. Due to how ERC-20 is
///    designed, it is not possible for the WETH contract to reject this transfer.
///    A significant amount of money has been lost due to this.
///
///    Dango solves this by introducing a `receive` entry point to every contract.
///    A contract can simply throw an error if it does not wish to accept a
///    specific transfer of tokens.
///
/// 2. **It is possible to send tokens to a non-existent recipient.**
///
///    This can happen if the sender makes a typo when inputting the recipient's
///    address. There will be no way to recover the tokens.
///
///    To solve this, the Dango bank contract checks whether the recipient exists
///    before executing the transfer. If the recipient doesn't exist, we call this
///    an "**orphaned transfer**". The tokens will be temporarily held in the bank
///    contract. Either the sender or the recipient (once it exists) can claim
///    the tokens by calling the `recover_transfer` method.
#[cfg_attr(not(feature = "library"), grug::export)]
pub fn bank_execute(ctx: SudoCtx, msg: BankMsg) -> anyhow::Result<Response> {
    let mut events = EventBuilder::with_capacity(msg.transfers.len() * 3);

    for (to, coins) in msg.transfers {
        // If the recipient exists, increase the recipient's balance. Otherwise,
        // 1. withhold the tokens in the bank contract;
        // 2. record the transfer in the `ORPHANED_TRANSFERS` map.
        let recipient_exists = ctx.querier.query_contract(to).is_ok();
        let recipient = if recipient_exists {
            to
        } else {
            ORPHANED_TRANSFERS.may_update(ctx.storage, (msg.from, to), |amount| {
                let mut amount = amount.unwrap_or_default();
                amount.insert_many(coins.clone())?;
                Ok::<_, StdError>(amount)
            })?;

            ctx.contract
        };

        for coin in &coins {
            decrease_balance(ctx.storage, &msg.from, coin.denom, *coin.amount)?;
            increase_balance(ctx.storage, &recipient, coin.denom, *coin.amount)?;
        }

        events
            .may_push(if !recipient_exists {
                Some(TransferOrphaned {
                    from: msg.from,
                    to,
                    coins: coins.clone(),
                })
            } else {
                None
            })?
            .push(Sent {
                user: msg.from,
                to: recipient,
                coins: coins.clone(),
            })?
            .push(Received {
                user: recipient,
                from: msg.from,
                coins,
            })?;
    }

    Ok(Response::new().add_events(events)?)
}

fn increase_supply(
    storage: &mut dyn Storage,
    denom: &Denom,
    amount: Uint128,
) -> anyhow::Result<Option<Uint128>> {
    SUPPLIES
        .may_modify(storage, denom, |maybe_supply| -> StdResult<_> {
            let supply = maybe_supply.unwrap_or(Uint128::ZERO).checked_add(amount)?;
            // Only write to storage if the supply is non-zero.
            if supply.is_zero() {
                Ok(None)
            } else {
                Ok(Some(supply))
            }
        })
        .map_err(|err| {
            anyhow!("failed to increase supply! denom: {denom}, amount: {amount}, reason: {err}")
        })
}

fn decrease_supply(
    storage: &mut dyn Storage,
    denom: &Denom,
    amount: Uint128,
) -> anyhow::Result<Option<Uint128>> {
    SUPPLIES
        .may_modify(storage, denom, |maybe_supply| -> StdResult<_> {
            let supply = maybe_supply.unwrap_or(Uint128::ZERO).checked_sub(amount)?;
            // If supply is reduced to zero, delete it, to save disk space.
            if supply.is_zero() {
                Ok(None)
            } else {
                Ok(Some(supply))
            }
        })
        .map_err(|err| {
            anyhow!("failed to decrease supply! denom: {denom}, amount: {amount}, reason: {err}")
        })
}

fn increase_balance(
    storage: &mut dyn Storage,
    address: &Addr,
    denom: &Denom,
    amount: Uint128,
) -> anyhow::Result<Option<Uint128>> {
    BALANCES
        .may_modify(storage, (address, denom), |maybe_balance| -> StdResult<_> {
            let balance = maybe_balance.unwrap_or(Uint128::ZERO).checked_add(amount)?;
            // Only write to storage if the balance is non-zero.
            if balance.is_zero() {
                Ok(None)
            } else {
                Ok(Some(balance))
            }
        })
        .map_err(|err| {
            anyhow!(
                "failed to increase balance! address: {address}, denom: {denom}, amount: {amount}, reason: {err}"
            )
        })
}

fn decrease_balance(
    storage: &mut dyn Storage,
    address: &Addr,
    denom: &Denom,
    amount: Uint128,
) -> anyhow::Result<Option<Uint128>> {
    BALANCES
        .may_modify(storage, (address, denom), |maybe_balance| -> StdResult<_> {
            let balance = maybe_balance.unwrap_or(Uint128::ZERO).checked_sub(amount)?;
            // If balance is reduced to zero, delete it, to save disk space.
            if balance.is_zero() {
                Ok(None)
            } else {
                Ok(Some(balance))
            }
        })
        .map_err(|err| {
            anyhow!(
                "failed to decrease balance! address: {address}, denom: {denom}, amount: {amount}, reason: {err}"
            )
        })
}
