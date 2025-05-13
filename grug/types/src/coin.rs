use {
    crate::{Denom, StdError, StdResult},
    borsh::{BorshDeserialize, BorshSerialize},
    grug_math::{IsZero, Uint128},
    serde::{Deserialize, Serialize},
    std::fmt,
};

/// An immutable reference to a coin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoinRef<'a> {
    pub denom: &'a Denom,
    pub amount: &'a Uint128,
}

/// A mutable reference to a coin.
///
/// Note that the denom isn't mutable; only the amount is.
#[derive(Debug, PartialEq, Eq)]
pub struct CoinRefMut<'a> {
    pub denom: &'a Denom,
    pub amount: &'a mut Uint128,
}

/// A coin, defined by a denomincation ("denom") and an amount.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Coin {
    pub denom: Denom,
    pub amount: Uint128,
}

impl Coin {
    /// Create a new `Coin` from the given denom and amount.
    pub fn new<D, A>(denom: D, amount: A) -> StdResult<Self>
    where
        D: TryInto<Denom>,
        A: Into<Uint128>,
        StdError: From<D::Error>,
    {
        Ok(Self {
            denom: denom.try_into()?,
            amount: amount.into(),
        })
    }

    /// Return an immutable reference to the coin.
    pub fn as_ref(&self) -> CoinRef {
        CoinRef {
            denom: &self.denom,
            amount: &self.amount,
        }
    }

    /// Return a mutable reference to the coin.
    pub fn as_mut(&mut self) -> CoinRefMut {
        CoinRefMut {
            denom: &self.denom,
            amount: &mut self.amount,
        }
    }
}

impl IsZero for Coin {
    fn is_zero(&self) -> bool {
        self.amount.is_zero()
    }
}

impl<D, A> TryFrom<(D, A)> for Coin
where
    D: TryInto<Denom>,
    A: Into<Uint128>,
    StdError: From<D::Error>,
{
    type Error = StdError;

    fn try_from((denom, amount): (D, A)) -> StdResult<Self> {
        Self::new(denom, amount)
    }
}

impl fmt::Display for Coin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.denom, self.amount)
    }
}

impl fmt::Debug for Coin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Coin({}:{})", self.denom, self.amount)
    }
}
