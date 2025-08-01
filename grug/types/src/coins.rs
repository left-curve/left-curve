use {
    crate::{Coin, CoinPair, CoinRef, Denom, Inner, NonZero, StdError, StdResult, btree_map},
    borsh::{BorshDeserialize, BorshSerialize},
    grug_math::{IsZero, Number, NumberConst, Uint128},
    serde::{Deserialize, Serialize},
    std::{
        collections::{BTreeMap, btree_map},
        fmt,
        str::FromStr,
    },
};

/// Build a [`Coins`](crate::Coins) with the given denoms and amounts.
///
/// Panic if input is invalid, e.g. invalid denom or zero amount(s).
#[macro_export]
macro_rules! coins {
    ($($denom:expr => $amount:expr),* $(,)?) => {{
        $crate::Coins::try_from($crate::btree_map! { $($denom => $amount),+ }).unwrap()
    }};
}

/// A sorted list of coins or tokens.
#[derive(
    Serialize, Deserialize, BorshSerialize, BorshDeserialize, Default, Clone, PartialEq, Eq,
)]
pub struct Coins(BTreeMap<Denom, Uint128>);

impl Coins {
    // There are two ways to stringify a Coins:
    //
    // 1. Use `grug::{to_json,from_json}`
    //    This is used in contract messages and responses.
    //    > [{"denom":"uatom","amount":"12345"},{"denom":"uosmo","amount":"67890"}]
    //
    // 2. Use `Coins::{to_string,from_str}`
    //    This is used in event logging and the CLI.
    //    > uatom:12345,uosmo:67890
    //
    // For method 2 specifically, an empty Coins stringifies to an empty string.
    // This can sometimes be confusing. Therefore we make this a special case
    // and stringifies it to a set of empty square brackets instead.
    pub const EMPTY_COINS_STR: &'static str = "[]";

    /// Create a new `Coins` without any coin.
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    /// Create a new `Coins` from an inner map without checking for validity.
    pub fn new_unchecked(inner: BTreeMap<Denom, Uint128>) -> Self {
        Self(inner)
    }

    /// Create a new `Coins` with exactly one coin.
    /// Error if the denom isn't valid, or amount is zero.
    pub fn one<D, A>(denom: D, amount: A) -> StdResult<Self>
    where
        D: TryInto<Denom>,
        A: Into<Uint128>,
        StdError: From<D::Error>,
    {
        let denom = denom.try_into()?;
        let amount = NonZero::new(amount.into())?;

        Ok(Self(btree_map! { denom => amount.into_inner() }))
    }

    /// Return whether the `Coins` contains any coin at all.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Return true if the `Coins` is non empty, false otherwise.
    pub fn is_non_empty(&self) -> bool {
        !self.0.is_empty()
    }

    /// Return the number of coins.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Iterate over the coins as immutable references.
    pub fn iter(&self) -> CoinsIter {
        self.into_iter()
    }

    /// Return whether there is a non-zero amount of the given denom.
    pub fn has(&self, denom: &Denom) -> bool {
        self.0.contains_key(denom)
    }

    /// Get the amount of the given denom.
    /// Note, if the denom does not exist, zero is returned.
    pub fn amount_of(&self, denom: &Denom) -> Uint128 {
        self.0.get(denom).copied().unwrap_or(Uint128::ZERO)
    }

    /// If the `Coins` is exactly one coin, return a reference to this coin;
    /// otherwise throw error.
    pub fn as_one_coin(&self) -> StdResult<CoinRef> {
        if self.0.len() != 1 {
            return Err(StdError::invalid_payment(1, self.len()));
        }

        let (denom, amount) = self.0.iter().next().unwrap();

        Ok(CoinRef { denom, amount })
    }

    /// If the `Coins` is exactly one coin, and is of the given denom, return a
    /// reference to this coin; otherwise throw error.
    pub fn as_one_coin_of_denom(&self, denom: &Denom) -> StdResult<CoinRef> {
        let coin = self.as_one_coin()?;

        if coin.denom != denom {
            return Err(StdError::invalid_payment(denom, coin.denom));
        }

        Ok(coin)
    }

    /// If the `Coins` is exactly one coin, consume self and return this coin as
    /// an owned value; otherwise throw error.
    pub fn into_one_coin(self) -> StdResult<Coin> {
        if self.0.len() != 1 {
            return Err(StdError::invalid_payment(1, self.len()));
        }

        let (denom, amount) = self.0.into_iter().next().unwrap();

        Ok(Coin { denom, amount })
    }

    /// If the `Coins` is exactly one coin, and is of the given denom, consume
    /// self and return this coin as an owned value; otherwise throw error.
    pub fn into_one_coin_of_denom(self, denom: &Denom) -> StdResult<Coin> {
        let coin = self.into_one_coin()?;

        if coin.denom != *denom {
            return Err(StdError::invalid_payment(denom, coin.denom));
        }

        Ok(coin)
    }

    /// If the `Coins` is exactly two coins, return these two coins as a tuple,
    /// sorted by denom; otherwise throw error.
    pub fn as_two_coins(&self) -> StdResult<(CoinRef, CoinRef)> {
        if self.0.len() != 2 {
            return Err(StdError::invalid_payment(2, self.len()));
        }

        let mut iter = self.0.iter();

        let (denom1, amount1) = iter.next().unwrap();
        let (denom2, amount2) = iter.next().unwrap();

        Ok((
            CoinRef {
                denom: denom1,
                amount: amount1,
            },
            CoinRef {
                denom: denom2,
                amount: amount2,
            },
        ))
    }

    /// If the `Coins` is exactly two coins, consume self and return these two
    /// coins as a tuple, sorted by denom; otherwise throw error.
    pub fn into_two_coins(self) -> StdResult<(Coin, Coin)> {
        if self.0.len() != 2 {
            return Err(StdError::invalid_payment(2, self.len()));
        }

        let mut iter = self.0.into_iter();

        let (denom1, amount1) = iter.next().unwrap();
        let (denom2, amount2) = iter.next().unwrap();

        Ok((
            Coin {
                denom: denom1,
                amount: amount1,
            },
            Coin {
                denom: denom2,
                amount: amount2,
            },
        ))
    }

    /// Insert a new coin to the `Coins`.
    pub fn insert<T>(&mut self, coin: T) -> StdResult<&mut Self>
    where
        T: TryInto<Coin>,
        StdError: From<T::Error>,
    {
        let coin = coin.try_into()?;

        let Some(amount) = self.0.get_mut(&coin.denom) else {
            // If the denom doesn't exist, and we are increasing by a non-zero
            // amount: just create a new record, and we are done.
            if coin.amount.is_non_zero() {
                self.0.insert(coin.denom, coin.amount);
            }

            return Ok(self);
        };

        amount.checked_add_assign(coin.amount)?;

        Ok(self)
    }

    /// Insert all coins from another `Coins`.
    pub fn insert_many<T>(&mut self, coins: T) -> StdResult<&mut Self>
    where
        T: IntoIterator<Item = Coin>,
    {
        for coin in coins {
            self.insert(coin)?;
        }

        Ok(self)
    }

    /// Deduct a coin from the `Coins`.
    pub fn deduct<T>(&mut self, coin: T) -> StdResult<&mut Self>
    where
        T: TryInto<Coin>,
        StdError: From<T::Error>,
    {
        let coin = coin.try_into()?;

        let Some(amount) = self.0.get_mut(&coin.denom) else {
            return Err(StdError::denom_not_found(coin.denom));
        };

        amount.checked_sub_assign(coin.amount)?;

        if amount.is_zero() {
            self.0.remove(&coin.denom);
        }

        Ok(self)
    }

    /// Deduct all coins from another `Coins`.
    pub fn deduct_many<T>(&mut self, coins: T) -> StdResult<&mut Self>
    where
        T: IntoIterator<Item = Coin>,
    {
        for coin in coins {
            self.deduct(coin)?;
        }

        Ok(self)
    }

    /// Deduct a coin from the `Coins`, saturating at zero. Returns a coin of
    /// the remainder if the coin's amount is greater than the available amount.
    pub fn saturating_deduct<T>(&mut self, coin: T) -> StdResult<Coin>
    where
        T: TryInto<Coin>,
        StdError: From<T::Error>,
    {
        let coin = coin.try_into()?;

        let Some(amount) = self.0.get_mut(&coin.denom) else {
            return Ok(coin);
        };

        if &coin.amount >= amount {
            let remainder = coin.amount - *amount;

            self.0.remove(&coin.denom);

            Ok(Coin {
                denom: coin.denom,
                amount: remainder,
            })
        } else {
            amount.checked_sub_assign(coin.amount)?;

            Ok(Coin {
                denom: coin.denom,
                amount: Uint128::ZERO,
            })
        }
    }

    /// Deduct all coins from another `Coins`, saturating at zero. Returns a
    pub fn saturating_deduct_many<T>(&mut self, coins: T) -> StdResult<Self>
    where
        T: IntoIterator<Item = Coin>,
    {
        let mut remainders = Self::new();

        for coin in coins {
            remainders.insert(self.saturating_deduct(coin)?)?;
        }

        Ok(remainders)
    }

    /// Take a coin of the given denom out of the `Coins`.
    /// Return a coin of zero amount if the denom doesn't exist in this `Coins`.
    pub fn take(&mut self, denom: Denom) -> Coin {
        let amount = self.0.remove(&denom).unwrap_or(Uint128::ZERO);

        Coin { denom, amount }
    }

    /// Take a pair of coins of the given denoms out of the `Coins`.
    /// Error if the two denoms are the same.
    pub fn take_pair(&mut self, (denom1, denom2): (Denom, Denom)) -> StdResult<CoinPair> {
        let amount1 = self.0.remove(&denom1).unwrap_or(Uint128::ZERO);
        let amount2 = self.0.remove(&denom2).unwrap_or(Uint128::ZERO);

        CoinPair::new(
            Coin {
                denom: denom1,
                amount: amount1,
            },
            Coin {
                denom: denom2,
                amount: amount2,
            },
        )
    }

    /// Convert an iterator over denoms and amounts to `Coins`.
    ///
    /// Used internally for implementing:
    ///
    /// - `TryFrom<[Coin; N]>`
    /// - `TryFrom<Vec<Coin>>`
    /// - `TryFrom<BTreeMap<String, Uint128>>`.
    ///
    /// Ensure the iterator doesn't contain duplicates. Zero amounts are skipped.
    fn try_from_iterator<D, A, I>(iter: I) -> StdResult<Self>
    where
        D: TryInto<Denom>,
        A: Into<Uint128>,
        I: IntoIterator<Item = (D, A)>,
        StdError: From<D::Error>,
    {
        let mut map = BTreeMap::new();

        for (denom, amount) in iter {
            let denom = denom.try_into()?;
            let amount = amount.into();

            if amount.is_zero() {
                continue;
            }

            if map.insert(denom, amount).is_some() {
                return Err(StdError::invalid_coins("duplicate denom: `{denom}`"));
            }
        }

        Ok(Self(map))
    }

    // note that we provide iter and into_iter methods, but not iter_mut method,
    // because users may use it to perform illegal actions, such as setting a
    // denom's amount to zero. use increase_amount and decrease_amount methods
    // instead.
}

impl Inner for Coins {
    type U = BTreeMap<Denom, Uint128>;

    fn inner(&self) -> &Self::U {
        &self.0
    }

    fn into_inner(self) -> Self::U {
        self.0
    }
}

// cast a string of the following format to Coins:
// denom1:amount1,denom2:amount2,...,denomN:amountN
// allow the denoms to be out of order, but disallow duplicates and zero amounts.
// this is mostly intended to use in CLIs.
impl FromStr for Coins {
    type Err = StdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // handle special case: empty string
        if s == Self::EMPTY_COINS_STR {
            return Ok(Coins::new());
        }

        let mut map = BTreeMap::new();

        for coin_str in s.split(',') {
            let Some((denom_str, amount_str)) = coin_str.split_once(':') else {
                return Err(StdError::invalid_coins(format!(
                    "invalid coin `{coin_str}`: must be in the format {{denom}}:{{amount}}"
                )));
            };

            let denom = Denom::from_str(denom_str)?;

            if map.contains_key(&denom) {
                return Err(StdError::invalid_coins(format!("duplicate denom: {denom}")));
            }

            let Ok(amount) = Uint128::from_str(amount_str) else {
                return Err(StdError::invalid_coins(format!(
                    "invalid amount `{amount_str}`"
                )));
            };

            if amount.is_zero() {
                return Err(StdError::invalid_coins(format!(
                    "denom `{denom}` as zero amount"
                )));
            }

            map.insert(denom, amount);
        }

        Ok(Self(map))
    }
}

impl<const N: usize> TryFrom<[Coin; N]> for Coins {
    type Error = StdError;

    fn try_from(array: [Coin; N]) -> StdResult<Self> {
        Self::try_from_iterator(array.into_iter().map(|coin| (coin.denom, coin.amount)))
    }
}

impl TryFrom<Vec<Coin>> for Coins {
    type Error = StdError;

    fn try_from(vec: Vec<Coin>) -> StdResult<Self> {
        Self::try_from_iterator(vec.into_iter().map(|coin| (coin.denom, coin.amount)))
    }
}

impl<D, A, const N: usize> TryFrom<[(D, A); N]> for Coins
where
    D: TryInto<Denom>,
    A: Into<Uint128>,
    StdError: From<D::Error>,
{
    type Error = StdError;

    fn try_from(array: [(D, A); N]) -> StdResult<Self> {
        Self::try_from_iterator(array)
    }
}

impl<D, A> TryFrom<BTreeMap<D, A>> for Coins
where
    D: TryInto<Denom>,
    A: Into<Uint128>,
    StdError: From<D::Error>,
{
    type Error = StdError;

    fn try_from(map: BTreeMap<D, A>) -> StdResult<Self> {
        Self::try_from_iterator(map)
    }
}

impl From<Coin> for Coins {
    fn from(coin: Coin) -> Self {
        Self([(coin.denom, coin.amount)].into())
    }
}

impl From<Coins> for Vec<Coin> {
    fn from(coins: Coins) -> Self {
        coins.into_iter().collect()
    }
}

impl<'a> IntoIterator for &'a Coins {
    type IntoIter = CoinsIter<'a>;
    type Item = CoinRef<'a>;

    fn into_iter(self) -> Self::IntoIter {
        CoinsIter(self.0.iter())
    }
}

impl IntoIterator for Coins {
    type IntoIter = CoinsIntoIter;
    type Item = Coin;

    fn into_iter(self) -> Self::IntoIter {
        CoinsIntoIter(self.0.into_iter())
    }
}

pub struct CoinsIter<'a>(btree_map::Iter<'a, Denom, Uint128>);

impl<'a> Iterator for CoinsIter<'a> {
    type Item = CoinRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0
            .next()
            .map(|(denom, amount)| CoinRef { denom, amount })
    }
}

pub struct CoinsIntoIter(btree_map::IntoIter<Denom, Uint128>);

impl Iterator for CoinsIntoIter {
    type Item = Coin;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(denom, amount)| Coin { denom, amount })
    }
}

impl fmt::Display for Coins {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // special case: empty string
        if self.is_empty() {
            return f.write_str(Self::EMPTY_COINS_STR);
        }

        let s = self
            .into_iter()
            .map(|coin| format!("{}:{}", coin.denom, coin.amount))
            .collect::<Vec<_>>()
            .join(",");

        f.write_str(&s)
    }
}

impl fmt::Debug for Coins {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Coins({self})")
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        crate::{Coins, Denom, Json, JsonDeExt, JsonSerExt, btree_map, json},
        grug_math::Uint128,
        std::str::FromStr,
    };

    fn mock_coins() -> Coins {
        Coins::new_unchecked(btree_map! {
            Denom::new_unchecked(["uatom"]) => Uint128::new(123),
            Denom::new_unchecked(["umars"]) => Uint128::new(456),
            Denom::new_unchecked(["uosmo"]) => Uint128::new(789),
        })
    }

    fn mock_coins_json() -> Json {
        json!({
            "uatom": "123",
            "umars": "456",
            "uosmo": "789",
        })
    }

    #[test]
    fn serializing_coins() {
        assert_eq!(mock_coins().to_json_value().unwrap(), mock_coins_json());
    }

    #[test]
    fn deserializing_coins() {
        // valid string
        assert_eq!(
            mock_coins_json().deserialize_json::<Coins>().unwrap(),
            mock_coins()
        );

        // invalid json: contains zero amount
        let illegal_json = json!([
            {
                "denom": "uatom",
                "amount": "0",
            },
        ]);
        assert!(illegal_json.deserialize_json::<Coins>().is_err());

        // invalid json: contains duplicate
        let illegal_json = json!([
            {
                "denom": "uatom",
                "amount": "123",
            },
            {
                "denom": "uatom",
                "amount": "456",
            },
        ]);
        assert!(illegal_json.deserialize_json::<Coins>().is_err());
    }

    #[test]
    fn coins_from_str() {
        // valid string. note: out of order is allowed
        let s = "uosmo:789,uatom:123,umars:456";
        assert_eq!(Coins::from_str(s).unwrap(), mock_coins());

        // invalid string: contains zero amount
        let s = "uatom:0";
        assert!(Coins::from_str(s).is_err());

        // invalid string: contains duplicate
        let s = "uatom:123,uatom:456";
        assert!(Coins::from_str(s).is_err())
    }

    #[test]
    fn saturating_deduct_many() {
        // Deduct less than available
        let mut coins = mock_coins();
        let deduct = Coins::new_unchecked(btree_map! {
            Denom::new_unchecked(["uatom"]) => Uint128::new(100),
            Denom::new_unchecked(["umars"]) => Uint128::new(100),
            Denom::new_unchecked(["uosmo"]) => Uint128::new(789),
        });
        let remainders = coins.saturating_deduct_many(deduct).unwrap();
        assert_eq!(remainders, Coins::new());

        // Equal amounts
        let mut coins = mock_coins();
        let remainders = coins.saturating_deduct_many(mock_coins()).unwrap();
        assert_eq!(remainders, Coins::new());

        // Some remainder
        let extra = Coins::new_unchecked(btree_map! {
            Denom::new_unchecked(["uatom"]) => Uint128::new(100),
            Denom::new_unchecked(["umars"]) => Uint128::new(100),
        });
        let mut coins = mock_coins();
        let mut deduct = mock_coins();
        deduct.insert_many(extra.clone()).unwrap();
        let remainders = coins.saturating_deduct_many(deduct).unwrap();
        assert_eq!(remainders, extra);

        // Deduct denom not in coins
        let mut coins = mock_coins();
        let deduct = Coins::new_unchecked(btree_map! {
            Denom::new_unchecked(["uatom"]) => Uint128::new(100),
            Denom::new_unchecked(["uusdc"]) => Uint128::new(100),
        });
        let remainders = coins.saturating_deduct_many(deduct).unwrap();
        assert_eq!(remainders, Coins::one("uusdc", 100).unwrap());
    }
}
