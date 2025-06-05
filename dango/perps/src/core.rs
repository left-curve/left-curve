use {
    anyhow::ensure,
    dango_types::perps::{
        INITIAL_SHARES_PER_TOKEN, PerpsMarketParams, PerpsMarketState, PerpsVaultState,
    },
    grug::{Denom, MultiplyRatio, Number, Sign, Signed, Udec128, Uint128},
    std::collections::HashMap,
};

pub fn token_to_shares(
    markets: &[PerpsMarketState],
    oracle_prices: &HashMap<Denom, Udec128>,
    params: &HashMap<Denom, PerpsMarketParams>,
    vault_state: &PerpsVaultState,
    amount: Uint128,
) -> anyhow::Result<Uint128> {
    let withdrawable_value = vault_state.withdrawable_value(markets, params, oracle_prices)?;

    // Calculate the amount of shares to mint
    let shares = if !withdrawable_value.is_positive() {
        amount.checked_mul(INITIAL_SHARES_PER_TOKEN)?
    } else {
        vault_state
            .shares
            .checked_multiply_ratio(amount, withdrawable_value.checked_into_unsigned()?)?
    };

    Ok(shares)
}

pub fn shares_to_token(
    markets: &[PerpsMarketState],
    oracle_prices: &HashMap<Denom, Udec128>,
    params: &HashMap<Denom, PerpsMarketParams>,
    vault_state: &PerpsVaultState,
    shares: Uint128,
) -> anyhow::Result<Uint128> {
    let withdrawable_value = vault_state.withdrawable_value(markets, params, oracle_prices)?;

    ensure!(
        withdrawable_value.is_positive(),
        "vault is undercollateralized"
    );

    let amount = withdrawable_value
        .checked_into_unsigned()?
        .checked_multiply_ratio(shares, vault_state.shares)?;

    Ok(amount)
}
