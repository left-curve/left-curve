use {
    dango_types::perps::{INITIAL_SHARES_PER_TOKEN, PerpsVaultState},
    grug::{IsZero, MultiplyRatio, Number, Uint128},
};

pub fn token_to_shares(vault_state: &PerpsVaultState, amount: Uint128) -> anyhow::Result<Uint128> {
    // Calculate the amount of shares to mint
    let shares = if vault_state.deposits.is_zero() {
        amount.checked_mul(INITIAL_SHARES_PER_TOKEN)?
    } else {
        vault_state
            .shares
            .checked_multiply_ratio(amount, vault_state.deposits)?
    };

    Ok(shares)
}

pub fn shares_to_token(vault_state: &PerpsVaultState, shares: Uint128) -> anyhow::Result<Uint128> {
    let amount = vault_state
        .deposits
        .checked_multiply_ratio(shares, vault_state.shares)?;

    Ok(amount)
}
