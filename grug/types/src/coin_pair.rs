use {
    crate::{Coin, CoinRef, CoinRefMut, Coins, Denom, StdError, StdResult},
    borsh::{BorshDeserialize, BorshSerialize},
    grug_math::{IsZero, MultiplyRatio, Number, NumberConst, Uint128},
    serde::{Serialize, de},
    std::{cmp::Ordering, collections::BTreeMap, io},
};

/// Build a [`CoinPair`](crate::CoinPair) with the given pair of denoms and amounts.
///
/// Panic if input is invalid, e.g. the two denoms are the same.
#[macro_export]
macro_rules! coin_pair {
    ($denom1:expr => $amount1:expr, $denom2:expr => $amount2:expr $(,)?) => {
        $crate::CoinPair::new(
            Coin::new($denom1, $amount1).unwrap(),
            Coin::new($denom2, $amount2).unwrap(),
        )
        .unwrap()
    };
}

/// A _sorted_ pair of coins of distinct denoms and possibly zero amounts.
#[derive(Serialize, BorshSerialize, Clone, Debug, PartialEq, Eq)]
pub struct CoinPair([Coin; 2]);

impl CoinPair {
    /// Create a new coin pair.
    /// Error if the two coins have the same denom.
    pub fn new(coin1: Coin, coin2: Coin) -> StdResult<Self> {
        match coin1.denom.cmp(&coin2.denom) {
            Ordering::Equal => Err(StdError::invalid_coins(format!(
                "coin pair with duplicate denom: {}",
                coin1.denom
            ))),
            Ordering::Less => Ok(Self([coin1, coin2])),
            Ordering::Greater => Ok(Self([coin2, coin1])),
        }
    }

    /// Create a new coin pair with zero amounts.
    pub fn new_empty(denom1: Denom, denom2: Denom) -> StdResult<Self> {
        Self::new(
            Coin::new(denom1, Uint128::ZERO)?,
            Coin::new(denom2, Uint128::ZERO)?,
        )
    }

    /// Create a new coin pair without checking whether the denoms are distinct.
    pub fn new_unchecked(coin1: Coin, coin2: Coin) -> Self {
        Self([coin1, coin2])
    }

    /// Return an immutable reference to the first coin.
    pub fn first(&self) -> CoinRef {
        self.0[0].as_ref()
    }

    /// Return a mutable reference to the first coin.
    pub fn first_mut(&mut self) -> CoinRefMut {
        self.0[0].as_mut()
    }

    /// Return an immutable reference to the second coin.
    pub fn second(&self) -> CoinRef {
        self.0[1].as_ref()
    }

    /// Return a mutable reference to the second coin.
    pub fn second_mut(&mut self) -> CoinRefMut {
        self.0[1].as_mut()
    }

    /// Return a pair of immutable references to the two coins.
    pub fn as_ref(&self) -> (CoinRef, CoinRef) {
        let coin1 = self.0[0].as_ref();
        let coin2 = self.0[1].as_ref();

        (coin1, coin2)
    }

    /// Return a pair of immutable references to the two coins, but in reverse order.
    pub fn as_ref_rev(&self) -> (CoinRef, CoinRef) {
        let coin1 = self.0[0].as_ref();
        let coin2 = self.0[1].as_ref();

        (coin2, coin1)
    }

    /// Return a pair of mutable references to the two coins.
    pub fn as_mut(&mut self) -> (CoinRefMut, CoinRefMut) {
        // Note: we can't do something like:
        //
        // ```rust
        // return (self.first_mut(), self.second_mut());
        // ```
        //
        // Because this involves two mutable borrows of `self.0`, which is not
        // allowed. Instead we create a single mutable iterator and extract the
        // two coins.
        let mut iter_mut = self.0.iter_mut();
        let coin1 = iter_mut.next().unwrap();
        let coin2 = iter_mut.next().unwrap();

        (
            CoinRefMut {
                denom: &coin1.denom,
                amount: &mut coin1.amount,
            },
            CoinRefMut {
                denom: &coin2.denom,
                amount: &mut coin2.amount,
            },
        )
    }

    /// Return a pair of mutable references to the two coins, but in reverse order.
    pub fn as_mut_rev(&mut self) -> (CoinRefMut, CoinRefMut) {
        let mut iter_mut = self.0.iter_mut();
        let coin1 = iter_mut.next().unwrap();
        let coin2 = iter_mut.next().unwrap();

        (
            CoinRefMut {
                denom: &coin2.denom,
                amount: &mut coin2.amount,
            },
            CoinRefMut {
                denom: &coin1.denom,
                amount: &mut coin1.amount,
            },
        )
    }

    /// Merge two coin pairs into one by summing up the amounts.
    /// Error if the pairs don't have exactly the same two denom.
    pub fn merge(&mut self, other: Self) -> StdResult<()> {
        if self.0[0].denom != other.0[0].denom || self.0[1].denom != other.0[1].denom {
            return Err(StdError::invalid_coins(format!(
                "can't merge coin pairs have different denoms: {}/{} != {}/{}",
                self.0[0].denom, self.0[1].denom, other.0[0].denom, other.0[1].denom
            )));
        }

        self.0[0].amount.checked_add_assign(other.0[0].amount)?;
        self.0[1].amount.checked_add_assign(other.0[1].amount)?;

        Ok(())
    }

    /// Split a portion of each of the given ratio from self.
    ///
    /// The numerator must be no greater than the denominator, otherwise a
    /// subtraction overflow error is returned.
    pub fn split(&mut self, numerator: Uint128, denominator: Uint128) -> StdResult<Self> {
        let amount1 = self.0[0]
            .amount
            .checked_multiply_ratio_floor(numerator, denominator)?;
        let amount2 = self.0[1]
            .amount
            .checked_multiply_ratio_floor(numerator, denominator)?;

        self.0[0].amount.checked_sub_assign(amount1)?;
        self.0[1].amount.checked_sub_assign(amount2)?;

        Ok(Self([
            Coin {
                denom: self.0[0].denom.clone(),
                amount: amount1,
            },
            Coin {
                denom: self.0[1].denom.clone(),
                amount: amount2,
            },
        ]))
    }

    /// Return true if the coin pair has the given denom.
    pub fn has(&self, denom: &Denom) -> bool {
        self.first().denom == denom || self.second().denom == denom
    }

    /// Return the amount of the given denom in the coin pair.
    pub fn amount_of(&self, denom: &Denom) -> StdResult<Uint128> {
        if self.first().denom == denom {
            Ok(*self.first().amount)
        } else if self.second().denom == denom {
            Ok(*self.second().amount)
        } else {
            Err(StdError::invalid_coins(format!(
                "coin pair {self:?} doesn't have denom: {denom}",
            )))
        }
    }

    /// Add a coin to the coin pair.
    ///
    /// Error if the coin pair doesn't have the denom or if the addition
    /// overflows.
    pub fn checked_add(&mut self, other: &Coin) -> StdResult<&mut Self> {
        if self.first().denom == &other.denom {
            self.first_mut().amount.checked_add_assign(other.amount)?;
        } else if self.second().denom == &other.denom {
            self.second_mut().amount.checked_add_assign(other.amount)?;
        } else {
            return Err(StdError::invalid_coins(format!(
                "can't add coin {other} to coin pair {self:?}"
            )));
        }

        Ok(self)
    }

    /// Subtract a coin from the coin pair.
    ///
    /// Error if the coin pair doesn't have the denom or if the subtraction
    /// underflows.
    pub fn checked_sub(&mut self, other: &Coin) -> StdResult<&mut Self> {
        if self.first().denom == &other.denom {
            self.first_mut().amount.checked_sub_assign(other.amount)?;
        } else if self.second().denom == &other.denom {
            self.second_mut().amount.checked_sub_assign(other.amount)?;
        } else {
            return Err(StdError::invalid_coins(format!(
                "can't subtract coin {other} from coin pair {self:?}"
            )));
        }

        Ok(self)
    }
}

impl TryFrom<Coins> for CoinPair {
    type Error = StdError;

    fn try_from(coins: Coins) -> StdResult<Self> {
        if coins.len() != 2 {
            return Err(StdError::invalid_coins("number of coins isn't exactly two"));
        }

        let mut iter = coins.into_iter();
        let coin1 = iter.next().unwrap();
        let coin2 = iter.next().unwrap();

        Ok(Self([coin1, coin2]))
    }
}

impl From<CoinPair> for Coins {
    fn from(pair: CoinPair) -> Self {
        let [coin1, coin2] = pair.0;

        let mut map = BTreeMap::new();
        if coin1.amount.is_non_zero() {
            map.insert(coin1.denom, coin1.amount);
        }
        if coin2.amount.is_non_zero() {
            map.insert(coin2.denom, coin2.amount);
        }

        Coins::new_unchecked(map)
    }
}

impl<'de> de::Deserialize<'de> for CoinPair {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        <[Coin; 2] as de::Deserialize>::deserialize(deserializer).map(CoinPair)
    }
}

impl BorshDeserialize for CoinPair {
    fn deserialize_reader<R>(reader: &mut R) -> io::Result<Self>
    where
        R: io::Read,
    {
        <[Coin; 2] as BorshDeserialize>::deserialize_reader(reader).map(CoinPair)
    }
}
