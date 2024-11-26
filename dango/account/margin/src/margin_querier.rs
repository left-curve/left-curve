use {
    dango_lending::DEBTS,
    dango_oracle::OracleQuerier,
    dango_types::{account::margin::HealthResponse, config::AppConfig},
    grug::{
        Addr, BorshDeExt, Coins, Inner, IsZero, Number, NumberConst, QuerierExt, QuerierWrapper,
        Udec128,
    },
};

/// Margin account query methods.
pub trait MarginQuerier {
    fn query_health(&self, account: Addr) -> anyhow::Result<HealthResponse>;
}

impl MarginQuerier for QuerierWrapper<'_> {
    fn query_health(&self, account: Addr) -> anyhow::Result<HealthResponse> {
        let app_cfg: AppConfig = self.query_app_config()?;

        // Query all debts for the account.
        let debts = self
            .query_wasm_raw(app_cfg.addresses.lending, DEBTS.path(account))?
            .map(|coins| coins.deserialize_borsh::<Coins>())
            .transpose()?
            .unwrap_or_default();

        // Calculate the total value of the debts.
        let mut total_debt_value = Udec128::ZERO;
        for debt in &debts {
            let price = self.query_price(app_cfg.addresses.oracle, debt.denom)?;
            let value = price.value_of_unit_amount(*debt.amount)?;

            total_debt_value.checked_add_assign(value)?;
        }

        // Calculate the total value of the account's collateral adjusted for the
        // collateral power.
        let mut total_collateral_value = Udec128::ZERO;
        let mut total_adjusted_collateral_value = Udec128::ZERO;
        for (denom, power) in app_cfg.collateral_powers {
            let collateral_balance = self.query_balance(account, denom.clone())?;

            // As an optimization, don't query the price if the collateral balance
            // is zero.
            if collateral_balance.is_zero() {
                continue;
            }

            let price = self.query_price(app_cfg.addresses.oracle, &denom)?;
            let value = price.value_of_unit_amount(collateral_balance)?;
            let adjusted_value = value.checked_mul(power.into_inner())?;

            total_collateral_value.checked_add_assign(value)?;
            total_adjusted_collateral_value.checked_add_assign(adjusted_value)?;
        }

        // Calculate the utilization rate.
        let utilization_rate = if total_debt_value.is_zero() {
            // The account has no debt. Utilization is zero in this case,
            // regardless of collateral value.
            Udec128::ZERO
        } else if total_adjusted_collateral_value.is_zero() {
            // The account has non-zero debt but zero collateral. This should
            // not happen! We set utilization to maximum.
            Udec128::MAX
        } else {
            total_debt_value / total_adjusted_collateral_value
        };

        Ok(HealthResponse {
            utilization_rate,
            total_debt_value,
            total_collateral_value,
            total_adjusted_collateral_value,
            debts,
        })
    }
}
