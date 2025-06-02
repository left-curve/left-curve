use grug::{
    Dec128, Denom, Inner, Int128, MathError, Number, NumberConst, Sign, Timestamp, Udec128,
    Uint128, Unsigned,
};

use super::{PerpsMarketParams, PerpsPosition, RealisedCashFlow};

/// Current state of a perps market.
#[grug::derive(Serde, Borsh)]
pub struct PerpsMarketState {
    /// The denom of the market.
    pub denom: Denom,
    /// The long open interest of the market, in market denom units.
    pub long_oi: Uint128,
    /// The short open interest of the market, in market denom units.
    pub short_oi: Uint128,
    /// The last time the market was updated.
    pub last_updated: Timestamp,
    /// The latest funding rate of the market as a daily rate.
    pub last_funding_rate: Dec128,
    /// Cumulative funding that has accrued so far, **in vault-denom
    /// per 1 base asset**.  Every position stores the value that was
    /// current at its last modification (`entry_funding_index`) so
    /// that funding-PnL = q · (global_index − entry_index).
    pub last_funding_index: Dec128,
    /// The perps market accumulators. Used to calculate the NAV of the vault.
    pub accumulators: PerpsMarketAccumulators,
    /// The realised cash flow of the market.
    pub realised_cash_flow: RealisedCashFlow,
}

/// Global, per-market accumulators — enable O(1) NAV calculation.
#[grug::derive(Serde, Borsh)]
pub struct PerpsMarketAccumulators {
    /// Σ q_k  (signed)
    pub net_position_q: Int128,

    /// Σ q_k * entry_execution_price_k  (signed, vault-denom units)
    pub cost_basis_sum: Dec128,

    /// Σ q_k * entry_funding_index_k  (signed, vault-denom units)
    pub funding_basis_sum: Dec128,

    /// Σ q_k * |q_k|  (signed, base-asset units)
    pub quadratic_fee_basis: Int128,
}

impl PerpsMarketAccumulators {
    pub fn new() -> Self {
        Self {
            net_position_q: Int128::ZERO,
            cost_basis_sum: Dec128::ZERO,
            funding_basis_sum: Dec128::ZERO,
            quadratic_fee_basis: Int128::ZERO,
        }
    }

    /// Decrease the accumulators by the given position.
    pub fn decrease(&mut self, position: &PerpsPosition) -> Result<(), MathError> {
        self.net_position_q = self.net_position_q.checked_sub(position.size)?;
        self.cost_basis_sum = self.cost_basis_sum.checked_sub(
            position
                .size
                .checked_into_dec()?
                .checked_mul(position.entry_execution_price)?,
        )?;
        self.funding_basis_sum = self.funding_basis_sum.checked_sub(
            position
                .size
                .checked_into_dec()?
                .checked_mul(position.entry_funding_index)?,
        )?;
        self.quadratic_fee_basis = self
            .quadratic_fee_basis
            .checked_sub(position.size.checked_mul(position.size.checked_abs()?)?)?;

        Ok(())
    }

    /// Increase the accumulators by the given position.
    pub fn increase(&mut self, position: &PerpsPosition) -> Result<(), MathError> {
        self.net_position_q = self.net_position_q.checked_add(position.size)?;
        self.cost_basis_sum = self.cost_basis_sum.checked_add(
            position
                .size
                .checked_into_dec()?
                .checked_mul(position.entry_execution_price)?,
        )?;
        self.funding_basis_sum = self.funding_basis_sum.checked_add(
            position
                .size
                .checked_into_dec()?
                .checked_mul(position.entry_funding_index)?,
        )?;
        self.quadratic_fee_basis = self
            .quadratic_fee_basis
            .checked_add(position.size.checked_mul(position.size.checked_abs()?)?)?;

        Ok(())
    }
}

impl PerpsMarketState {
    pub fn skew(&self) -> Result<Int128, MathError> {
        self.long_oi
            .checked_into_signed()?
            .checked_sub(self.short_oi.checked_into_signed()?)
    }

    /// Returns the pSkew = skew / skewScale capping the pSkew between [-1, 1].
    pub fn proportional_skew(&self, skew_scale: Uint128) -> Result<Dec128, MathError> {
        Ok(Dec128::checked_from_ratio(
            self.skew()?,
            skew_scale.checked_into_signed()?.into_inner(),
        )?
        .clamp(-Dec128::ONE, Dec128::ONE))
    }

    /// Returns the unrealized price PnL of the market.
    ///
    /// $$
    /// \text{pricePnL}(p)=
    /// K\,p
    /// - C
    /// +\frac{p}{2\,\text{skewScale}}
    ///   \Bigl(K^{2}-\tfrac12 S^{2}\Bigr)
    /// \tag{1}
    /// $$
    ///
    /// pricePnL(p) = Kp - C + p / (2 * skewScale) * (K^2 - S^2 / 2)
    pub fn unrealized_price_pnl(
        &self,
        oracle_price: Udec128,
        skew_scale: Uint128,
    ) -> Result<Int128, MathError> {
        let oracle_price = oracle_price.checked_into_signed()?;
        let K = self.accumulators.net_position_q.checked_into_dec()?;
        let C = self.accumulators.cost_basis_sum; // already Dec128
        let S2 = self.accumulators.quadratic_fee_basis; // Int128  (we still call it A here)
        let price_pnl = K * oracle_price               // K·p
    - C                               // −Σ q·p_entry
    + oracle_price
        * (K*K - S2.checked_into_dec()? / Dec128::new(2)) // correction term
        / (skew_scale.checked_into_dec()?.checked_into_signed()? * Dec128::new(2));
        Ok(price_pnl.into_int())
    }

    /// Returns the unrealized funding PnL of the market.
    /// $$
    /// \text{fundingPnL}(F_{\text{now}})=
    /// K\,F_{\text{now}} - F_\Sigma
    /// \tag{2}
    /// $$
    /// fundingPnL(F_now) = K * F_now - F_Sigma
    pub fn unrealized_funding_pnl(&self) -> Result<Int128, MathError> {
        let F_now = self.last_funding_index;
        let K = self.accumulators.net_position_q.checked_into_dec()?;
        let funding_pnl = K * F_now - self.accumulators.funding_basis_sum;
        Ok(funding_pnl.into_int())
    }

    /// Returns the unrealized fees of the market.
    /// $$
    /// \text{unrealisedFee}(p)=
    /// p\;\bigl(\varphi_m\,M + \varphi_t\,T\bigr)
    /// $$
    /// unrealisedFee(p) = p * (φ_m * M + φ_t * T)
    ///
    /// with $M,T$ from (★).
    pub fn unrealized_fees(
        &self,
        oracle_price: Udec128,
        maker_rate: Udec128,
        taker_rate: Udec128,
    ) -> Result<Int128, MathError> {
        let oracle_price = oracle_price.checked_into_signed()?;
        let K = self.accumulators.net_position_q.checked_into_dec()?;
        let S2 = self.accumulators.quadratic_fee_basis.checked_into_dec()?;
        let abs_K = K.checked_abs()?;
        let m_usd = (S2 - K * abs_K) / Dec128::new(2);
        let t_usd = (S2 + K * abs_K) / Dec128::new(2) - abs_K;
        let unreal_fee = oracle_price
            * (m_usd * maker_rate.checked_into_signed()?
                + t_usd * taker_rate.checked_into_signed()?);
        Ok(unreal_fee.into_int())
    }

    pub fn net_asset_value(
        &self,
        params: &PerpsMarketParams,
        oracle_price: Udec128,
    ) -> anyhow::Result<Int128> {
        let price_pnl = self.unrealized_price_pnl(oracle_price, params.skew_scale)?;
        let funding_pnl = self.unrealized_funding_pnl()?;
        let unrealized_fees = self.unrealized_fees(
            oracle_price,
            params.maker_fee.into_inner(),
            params.taker_fee.into_inner(),
        )?;
        Ok(self
            .realised_cash_flow
            .total()?
            .checked_add(price_pnl)?
            .checked_add(funding_pnl)?
            .checked_add(unrealized_fees)?)
    }
}
