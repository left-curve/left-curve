use {
    crate::{BALANCES, METADATAS, NAMESPACE_OWNERS, ORPHANED_TRANSFERS, PERP_DEFICIT, SUPPLIES},
    anyhow::{anyhow, bail, ensure},
    dango_oracle::OracleQuerier,
    dango_perps::{NoCachePerpQuerier, USER_STATES},
    dango_types::{
        bank::{
            Burned, ExecuteMsg, InstantiateMsg, Metadata, Minted, Received, Sent, TransferOrphaned,
        },
        perps::{self, settlement_currency},
    },
    grug::{
        Addr, BankMsg, Coins, Denom, EventBuilder, IsZero, MutableCtx, Number, NumberConst, Part,
        QuerierExt, Response, StdError, StdResult, Storage, StorageQuerier, SudoCtx, Uint128, addr,
    },
    std::{collections::HashMap, ops::Deref},
};

/// Address of the perps contract.
// TODO: update with the actual deployed perps contract address.
const PERPS: Addr = addr!("0000000000000000000000000000000000000000");

/// Address of the oracle contract.
const ORACLE: Addr = addr!("cedc5f73cbb963a48471b849c3650e6e34cd3b6d");

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
                "only chain owner can mint, burn, or set metadata of top-level denoms"
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
        ctx.sender == sender
            || ctx.sender == recipient
            || ctx.sender == ctx.querier.query_owner()?,
        "only the sender, the recipient, or the chain owner can recover an orphaned transfer"
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
    // ---------------------------- 1. Margin check ----------------------------

    // If the tokens being transferred include the perps settlement currency,
    // we need to ensure the amount being transferred is no more than the user's
    // available margin.

    // Sum settlement currency being transferred out across all recipients.
    let total_settlement = msg
        .transfers
        .values()
        .try_fold(Uint128::ZERO, |acc, coins| {
            acc.checked_add(coins.amount_of(&settlement_currency::DENOM))
        })?;

    // If the sender is transferring settlement currency, ensure they have
    // sufficient available margin to cover the transfer.
    if total_settlement.is_non_zero()
        && let Some(user_state) = ctx
            .querier
            .may_query_wasm_path(PERPS, &USER_STATES.path(msg.from))?
        && !user_state.positions.is_empty()
    {
        let perp_querier = NoCachePerpQuerier::new_remote(PERPS, ctx.querier);
        let mut oracle_querier = OracleQuerier::new_remote(ORACLE, ctx.querier);

        let balance = BALANCES
            .may_load(ctx.storage, (&msg.from, &settlement_currency::DENOM))?
            .unwrap_or(Uint128::ZERO);

        crate::perp_margin::check_perps_margin(
            &user_state,
            balance,
            total_settlement,
            &perp_querier,
            &mut oracle_querier,
        )?;
    }

    // --------------------------- 2. Balance update ---------------------------

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

        // Special rule if the address is the perps contract and the denom is
        // its settlement currency.
        //
        // When sending, we allow the perp contract to "overdraw" its balance.
        // If perp contract's balance is less than the amount it's sending, we
        // reduce its balance to zero and add the shortfall to the `PERP_DEFICIT`
        // storage slot. Conversely, when the perp contract receives a transfer,
        // we first reduce `PERP_DEFICIT` (if any) and only add the remainder to
        // its balance.
        //
        // To understand why, the context: in perpetual futures, whenever there
        // is a winner, there is necessary a loser. When the winner realizes his
        // positive PnL, the perp contract must pay out settlement currency tokens
        // from its balance to the user. Conversely, when a loser realizes his
        // negative PnL, he must pay the contract. The contract acts as a middle
        // man coordinating the flow of PnL between the parties. However, when
        // the winner realizes his PnL, the loser may not want to do it yet. Thus,
        // the contract may not have sufficient token to pay the winner. In is
        // case, we allow it to overdraw. The deficit should be temporarily, as
        // the loser has to realize his loss at one point (either closing voluntarily
        // or getting liquidated).
        for coin in &coins {
            if msg.from == PERPS && coin.denom == perps::settlement_currency::DENOM.deref() {
                decrease_perp_balance(ctx.storage, *coin.amount)?;
            } else {
                decrease_balance(ctx.storage, &msg.from, coin.denom, *coin.amount)?;
            }

            if recipient == PERPS && coin.denom == perps::settlement_currency::DENOM.deref() {
                increase_perp_balance(ctx.storage, *coin.amount)?;
            } else {
                increase_balance(ctx.storage, &recipient, coin.denom, *coin.amount)?;
            }
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

/// Increases the perps contract's settlement currency balance, repaying any
/// outstanding deficit first.
fn increase_perp_balance(storage: &mut dyn Storage, amount: Uint128) -> anyhow::Result<()> {
    let deficit = PERP_DEFICIT.may_load(storage)?.unwrap_or(Uint128::ZERO);

    let absorbed = amount.min(deficit);
    let remainder = amount.checked_sub(absorbed)?;

    // Reduce the deficit by the absorbed amount. Remove if zero.
    let new_deficit = deficit.checked_sub(absorbed)?;
    if new_deficit.is_zero() {
        PERP_DEFICIT.remove(storage);
    } else {
        PERP_DEFICIT.save(storage, &new_deficit)?;
    }

    // Credit the remainder to the balance.
    if remainder.is_non_zero() {
        let balance = BALANCES
            .may_load(storage, (&PERPS, &settlement_currency::DENOM))?
            .unwrap_or(Uint128::ZERO)
            .checked_add(remainder)?;
        BALANCES.save(storage, (&PERPS, &settlement_currency::DENOM), &balance)?;
    }

    Ok(())
}

/// Decreases the perps contract's settlement currency balance, allowing
/// overdraw by tracking the shortfall in `PERP_DEFICIT`.
fn decrease_perp_balance(storage: &mut dyn Storage, amount: Uint128) -> anyhow::Result<()> {
    let balance = BALANCES
        .may_load(storage, (&PERPS, &settlement_currency::DENOM))?
        .unwrap_or(Uint128::ZERO);

    let absorbed = amount.min(balance);
    let remainder = amount.checked_sub(absorbed)?;

    // Reduce the balance by the absorbed amount. Remove if zero.
    let new_balance = balance.checked_sub(absorbed)?;
    if new_balance.is_zero() {
        BALANCES.remove(storage, (&PERPS, &settlement_currency::DENOM));
    } else {
        BALANCES.save(storage, (&PERPS, &settlement_currency::DENOM), &new_balance)?;
    }

    // Add the remainder to the deficit.
    if remainder.is_non_zero() {
        let deficit = PERP_DEFICIT
            .may_load(storage)?
            .unwrap_or(Uint128::ZERO)
            .checked_add(remainder)?;
        PERP_DEFICIT.save(storage, &deficit)?;
    }

    Ok(())
}
