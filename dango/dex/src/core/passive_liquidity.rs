use {
    anyhow::ensure,
    dango_types::dex::{CurveInvariant, Direction, Pool, Swap},
    grug::{
        Coin, CoinPair, Denom, Int, IsZero, MultiplyFraction, MultiplyRatio, Number, NumberConst,
        Udec128, Uint128,
    },
};

pub trait PassiveLiquidityPool {
    /// Initialize the pool with the given reserves.
    ///
    /// Returns a tuple of the pool and the initial LP token supply.
    fn initialize(
        base_denom: Denom,
        quote_denom: Denom,
        reserves: CoinPair,
        curve_type: CurveInvariant,
        swap_fee: Udec128,
    ) -> anyhow::Result<(Box<Self>, Uint128)>;

    /// Provide liquidity to the pool. This function mutates the pool reserves.
    /// Liquidity is provided at the current pool balance, any excess funds are
    /// returned to the user.
    ///
    /// Returns a tuple of (`Udec128`, `CoinPair`) where the first element is the
    /// percentage by which to increase the LP token supply and the second element
    /// is the amount of each asset that was not used to provide liquidity.
    fn add_liquidity(&mut self, funds: CoinPair) -> anyhow::Result<Udec128>;

    /// Remove a portion of the liquidity from the pool. This function mutates the pool reserves.
    ///
    /// Returns the underlying liquidity that was removed.
    fn remove_liquidity(
        &mut self,
        numerator: Uint128,
        denominator: Uint128,
    ) -> anyhow::Result<CoinPair>;
}

pub trait TradingFunction {
    /// Calculate the value of the trading invariant.
    fn invariant(&self, reserves: &CoinPair) -> anyhow::Result<Uint128>;

}

impl PassiveLiquidityPool for Pool {
    fn initialize(
        base_denom: Denom,
        quote_denom: Denom,
        reserves: CoinPair,
        curve_type: CurveInvariant,
        swap_fee: Udec128,
    ) -> anyhow::Result<(Box<Self>, Uint128)> {
        ensure!(
            reserves.first().amount.is_non_zero() && reserves.second().amount.is_non_zero(),
            "cannot initialize pool with zero reserves"
        );

        ensure!(
            reserves.has(&base_denom) && reserves.has(&quote_denom),
            "invalid reserves"
        );

        let initial_lp_supply = match curve_type {
            CurveInvariant::Xyk => curve_type.invariant(&reserves)?.checked_sqrt()?,
        };

        Ok((
            Box::new(Self {
                base_denom,
                quote_denom,
                reserves,
                curve_type,
                swap_fee,
            }),
            initial_lp_supply,
        ))
    }

    fn add_liquidity(&mut self, funds: CoinPair) -> anyhow::Result<Udec128> {
        ensure!(
            self.reserves.first().amount.is_non_zero()
                && self.reserves.second().amount.is_non_zero(),
            "cannot add liquidity to pool with zero reserves"
        );

        ensure!(
            funds.first().denom == self.reserves.first().denom
                && funds.second().denom == self.reserves.second().denom,
            "invalid funds"
        );

        ensure!(
            funds.first().amount.is_non_zero() && funds.second().amount.is_non_zero(),
            "cannot add zero liquidity"
        );

        let invariant_before = self.curve_type.invariant(&self.reserves)?;

        println!("invariant_before: {}", invariant_before);

        // Add the used funds to the pool reserves
        self.reserves
            .checked_add(&Coin::new(
                funds.first().denom.clone(),
                funds.first().amount.clone(),
            )?)?
            .checked_add(&Coin::new(
                funds.second().denom.clone(),
                funds.second().amount.clone(),
            )?)?;

        // Compute the proportional increase in the invariant
        let invariant_after = self.curve_type.invariant(&self.reserves)?;
        let invariant_ratio = Udec128::checked_from_ratio(invariant_after, invariant_before)?;

        println!("invariant_after: {}", invariant_after);
        println!("invariant_ratio: {}", invariant_ratio);

        // Compute the mint ratio from the invariant ratio based on the curve type.
        // This ensures that an unbalances provision will be equivalent to a swap
        // followed by a balancedliquidity provision.
        let mint_ratio = match self.curve_type {
            CurveInvariant::Xyk => invariant_ratio.checked_sqrt()?,
        }
        .checked_sub(Udec128::ONE)?;

        println!("mint_ratio: {}", mint_ratio);

        Ok(mint_ratio)
    }

    fn remove_liquidity(
        &mut self,
        numerator: Uint128,
        denominator: Uint128,
    ) -> anyhow::Result<CoinPair> {
        Ok(self.reserves.split(numerator, denominator)?)
    }

}

impl TradingFunction for CurveInvariant {
    fn invariant(&self, reserves: &CoinPair) -> anyhow::Result<Uint128> {
        match self {
            CurveInvariant::Xyk => {
                Ok(reserves.first().amount.clone() * reserves.second().amount.clone())
            },
        }
    }
}
