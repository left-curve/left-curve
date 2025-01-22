use {
    dango_lending::DEBTS,
    dango_oracle::OracleQuerier,
    dango_types::{account::margin::HealthResponse, config::AppConfig},
    grug::{
        Addr, Coin, Coins, Inner, IsZero, Number, NumberConst, QuerierExt, StdError,
        StorageQuerier, Udec128,
    },
};

/// Margin account query methods.
pub trait MarginQuerier {
    /// Queries the health of the margin account.
    ///
    /// Arguments:
    ///
    /// - `account`: The margin account to query.
    /// - `discount_collateral`: If set, does not include the value of these
    ///    coins in the total collateral value. Used when liquidating the
    ///    account as the liquidator has sent additional funds to the account
    ///    that should not be included in the total collateral value.
    fn query_health(
        &self,
        account: Addr,
        discount_collateral: Option<Coins>,
    ) -> anyhow::Result<HealthResponse>;
}

impl<Q> MarginQuerier for Q
where
    Q: QuerierExt,
    Q::Error: From<StdError>,
    anyhow::Error: From<Q::Error>,
{
    fn query_health(
        &self,
        account: Addr,
        discount_collateral: Option<Coins>,
    ) -> anyhow::Result<HealthResponse> {
        let app_cfg: AppConfig = self.query_app_config()?;

        // Query all debts for the account.
        let debts = self
            .may_query_wasm_path(app_cfg.addresses.lending, DEBTS.path(account))?
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
        let mut collaterals = Coins::new();

        for (denom, power) in app_cfg.collateral_powers {
            let mut collateral_balance = self.query_balance(account, denom.clone())?;

            if let Some(discount_collateral) = discount_collateral.as_ref() {
                collateral_balance.checked_sub_assign(discount_collateral.amount_of(&denom))?;
            }

            // As an optimization, don't query the price if the collateral balance
            // is zero.
            if collateral_balance.is_zero() {
                continue;
            }

            let price = self.query_price(app_cfg.addresses.oracle, &denom)?;
            let value = price.value_of_unit_amount(collateral_balance)?;
            let adjusted_value = value.checked_mul(power.into_inner())?;

            collaterals.insert(Coin::new(denom, collateral_balance)?)?;
            total_collateral_value.checked_add_assign(value)?;
            total_adjusted_collateral_value.checked_add_assign(adjusted_value)?;
        }

        // Calculate the utilization rate.
        let utilization_rate = if total_debt_value.is_zero() {
            // The account has no debt. Utilization is zero in this case,
            // regardless of collateral value.
            Udec128::ZERO
        } else if total_adjusted_collateral_value.is_zero() {
            // The account has non-zero debt but zero collateral. This can
            // happen if the account is liquidated. We set utilization to
            // maximum.
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
            collaterals,
        })
    }
}
