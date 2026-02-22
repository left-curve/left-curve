use {
    crate::{NoCachePairQuerier, PAIR_STATES, STATE, core::compute_vault_equity},
    anyhow::ensure,
    dango_oracle::OracleQuerier,
    dango_types::{
        BaseAmount, FromInner, UsdValue, bank,
        perps::{self, State, settlement_currency},
    },
    grug::{
        Addr, Coins, Message, MutableCtx, Order as IterationOrder, Response, StdResult, Storage,
        Timestamp, addr,
    },
};

/// Virtual shares added to total supply in share price calculations.
/// Prevents the first-depositor attack (ERC-4626 inflation attack) by
/// ensuring the share price cannot be trivially inflated.
const VIRTUAL_SHARES: BaseAmount = BaseAmount::new(1_000_000);

/// Virtual assets added to vault equity in share price calculations.
/// Works in tandem with `VIRTUAL_SHARES` to set the initial share price
/// and prevent share inflation attacks.
const VIRTUAL_ASSETS: UsdValue = UsdValue::new(1);

/// Address of the bank contract.
const BANK: Addr = addr!("e0b49f70991ecab05d5d7dc1f71e4ede63c8f2b7");

/// Address of the oracle contract.
const ORACLE: Addr = addr!("cedc5f73cbb963a48471b849c3650e6e34cd3b6d");

pub fn deposit(
    ctx: MutableCtx,
    min_shares_to_mint: Option<BaseAmount>,
) -> anyhow::Result<Response> {
    // Load state, create querier objects.
    let mut state = STATE.load(ctx.storage)?;
    let pair_querier = NoCachePairQuerier::new_local(ctx.storage);
    let mut oracle_querier = OracleQuerier::new_remote(ORACLE, ctx.querier);

    // Run the deposit logic.
    let (deposit_amount, shares_to_mint) = _deposit(
        ctx.storage,
        ctx.block.timestamp,
        ctx.funds,
        &state,
        &pair_querier,
        &mut oracle_querier,
        min_shares_to_mint,
    )?;

    // Update global state.
    state.vault_margin.checked_add_assign(deposit_amount)?;

    // Save the updated global state.
    STATE.save(ctx.storage, &state)?;

    // Send a message to instruct the bank contract to mint the share token.
    // Note: if `shares_to_mint` is zero, the `Coins::one` constructor call errors,
    // as intended.
    Ok(Response::new().add_message(Message::execute(
        BANK,
        &bank::ExecuteMsg::Mint {
            to: ctx.sender,
            coins: Coins::one(perps::DENOM.clone(), shares_to_mint)?,
        },
        Coins::new(),
    )?))
}

/// The actual logic for handling the deposit.
/// Returns: 1) the amount of settlement currency that was deposited,
/// 2) the amount of share token to be minted, both in base unit.
fn _deposit(
    storage: &dyn Storage,
    current_time: Timestamp,
    mut funds: Coins,
    state: &State,
    pair_querier: &NoCachePairQuerier,
    oracle_querier: &mut OracleQuerier,
    min_shares_to_mint: Option<BaseAmount>,
) -> anyhow::Result<(BaseAmount, BaseAmount)> {
    // Query the price of the settlement currency.
    let settlement_currency_price =
        oracle_querier.query_price_for_perps(&settlement_currency::DENOM)?;

    // ------------------------- Step 1. Check deposit -------------------------

    // Find how much settlement currency the user has deposited.
    let deposit_amount =
        BaseAmount::from_inner(funds.take(settlement_currency::DENOM.clone()).amount);

    // The user should not have deposited anything else.
    ensure!(funds.is_empty(), "unexpected deposit: {:?}", funds);

    // --------------------- Step 2. Compute vault equity ----------------------

    // Add virtual shares to the current vault share supply to arrive at the
    // effective supply.
    let effective_supply = state.vault_share_supply.checked_add(VIRTUAL_SHARES)?;

    // Compute the value of the vault's margin by multiplying its balance with
    // the settlement currency price.
    let vault_margin_value = state
        .vault_margin
        .checked_into_human(settlement_currency::DECIMAL)?
        .checked_mul(settlement_currency_price)?;

    // Find all the existing trading pairs.
    // TODO: optimize this. Ideally we don't do database iteration which is slow.
    let pair_ids = PAIR_STATES
        .keys(storage, None, None, IterationOrder::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    // Compute the vault's equity. This equals the vault's margin plus its
    // unrealized PnL and funding.
    let vault_equity = compute_vault_equity(
        vault_margin_value,
        &pair_ids,
        pair_querier,
        oracle_querier,
        current_time,
    )?;

    // Add virtual asset to vault equity to arrive at the effective equity.
    let effective_equity = vault_equity.checked_add(VIRTUAL_ASSETS)?;

    ensure!(
        effective_equity.is_positive(),
        "vault is in catastrophic loss! deposit disabled. effective equity: {effective_equity}"
    );

    // -------------------------- Step 3. Mint shares --------------------------

    // Compute the value of the settlement currency the user is depositing.
    let deposit_value = deposit_amount
        .checked_into_human(settlement_currency::DECIMAL)?
        .checked_mul(settlement_currency_price)?;

    // Compute the amount of shares to mint.
    let shares_to_mint =
        effective_supply.checked_mul_ratio_floor(deposit_value.checked_div(effective_equity)?)?;

    if let Some(min_shares_to_mint) = min_shares_to_mint {
        ensure!(
            shares_to_mint >= min_shares_to_mint,
            "too few shares minted: {shares_to_mint} (actual) < {min_shares_to_mint} (expected)"
        );
    }

    Ok((deposit_amount, shares_to_mint))
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    // TODO
}
