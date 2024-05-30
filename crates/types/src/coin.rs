use {
    crate::{StdError, StdResult, Uint128},
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
    std::{
        collections::{btree_map, BTreeMap},
        fmt,
        str::FromStr,
    },
};

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Coin {
    pub denom: String,
    pub amount: Uint128,
}

impl Coin {
    pub fn new(denom: impl Into<String>, amount: impl Into<Uint128>) -> Self {
        Self {
            denom: denom.into(),
            amount: amount.into(),
        }
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

/// A record in the `Coins` map.
///
/// In `Coins`, we don't store coins an a vector of `Coin`s, but rather as
/// mapping from denoms to amounts. This ensures that there is no duplicate
/// denoms, and that coins are ordered by denoms alphabetically.
///
/// However, this also means that when we iterate records in the map, we don't
/// get a `&Coin`, but get a tuple `(&String, &Uint128)` which is less ergonomic
/// to work with.
///
/// We can of course create a temporary `Coin` value, but it would then require
/// cloning/dereferencing the denom and amount, which can be expensive.
///
/// Therefore, we create this struct which holds references to the denom and
/// amount.
#[derive(Serialize)]
pub struct CoinRef<'a> {
    pub denom: &'a String,
    pub amount: &'a Uint128,
}

#[derive(
    Serialize, Deserialize, BorshSerialize, BorshDeserialize, Default, Clone, PartialEq, Eq,
)]
pub struct Coins(BTreeMap<String, Uint128>);

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

    pub fn new_empty() -> Self {
        Self(BTreeMap::new())
    }

    pub fn new_one(denom: impl Into<String>, amount: impl Into<Uint128>) -> Self {
        Self([(denom.into(), amount.into())].into())
    }

    /// Cast an `Vec<Coin>` into a `Coins` object, without checking for
    /// duplicate denoms or zero amounts.
    /// This is potentially unsafe, intended for using in tests. Only use if you
    /// know what you're doing.
    #[doc(hidden)]
    pub fn from_vec_unchecked(vec: Vec<Coin>) -> Self {
        Self(
            vec.into_iter()
                .map(|coin| (coin.denom, coin.amount))
                .collect(),
        )
    }

    /// Collect an iterator over (denom, amount) tuples into a `Coins` object,
    /// without checking for duplicate denoms or zero amounts.
    /// This is solely intended for use in implementing the bank contract,
    /// where we know for sure there's no such illegal cases.\
    #[doc(hidden)]
    pub fn from_iter_unchecked<E>(
        iter: &mut dyn Iterator<Item = Result<(String, Uint128), E>>,
    ) -> Result<Self, E> {
        iter.collect::<Result<_, E>>().map(Self)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Return whether there is a non-zero amount of the given denom.
    pub fn has(&self, denom: &str) -> bool {
        self.0.contains_key(denom)
    }

    /// Get the amount of the given denom.
    /// Note, if the denom does not exist, zero is returned.
    pub fn amount_of(&self, denom: &str) -> Uint128 {
        self.0.get(denom).copied().unwrap_or(Uint128::ZERO)
    }

    /// Do nothing if the `Coins` is empty; throw an error if not empty.
    pub fn assert_empty(&self) -> StdResult<()> {
        if !self.is_empty() {
            return Err(StdError::payment(0, self.len()));
        }

        Ok(())
    }

    /// If the `Coins` is exactly one coin, return a reference to this coin;
    /// otherwise throw error.
    pub fn one_coin(&self) -> StdResult<CoinRef> {
        let Some((denom, amount)) = self.0.first_key_value() else {
            return Err(StdError::payment(1, 0));
        };

        if self.0.len() > 1 {
            return Err(StdError::payment(1, self.len()));
        }

        Ok(CoinRef { denom, amount })
    }

    /// Increase the amount of a denom by the given amount. If the denom doesn't
    /// exist, a new record is created.
    pub fn increase_amount(&mut self, denom: &str, by: Uint128) -> StdResult<()> {
        let Some(amount) = self.0.get_mut(denom) else {
            // if the denom doesn't exist, we just create a new record, and we
            // are done.
            self.0.insert(denom.into(), by);
            return Ok(());
        };

        *amount = amount.checked_add(by)?;

        Ok(())
    }

    /// Decrease the amount of a denom by the given amount. Amount can't be
    /// reduced below zero. If the amount is reduced to exactly zero, the record
    /// is purged, so that only non-zero amount coins remain.
    pub fn decrease_amount(&mut self, denom: &str, by: Uint128) -> StdResult<()> {
        let Some(amount) = self.0.get_mut(denom) else {
            return Err(StdError::DenomNotFound {
                denom: denom.into(),
            });
        };

        // TODO
        *amount = amount.checked_sub(by)?;

        if amount.is_zero() {
            self.0.remove(denom);
        }

        Ok(())
    }

    // note that we provide iter and into_iter methods, but not iter_mut method,
    // because users may use it to perform illegal actions, such as setting a
    // denom's amount to zero. use increase_amount and decrease_amount methods
    // instead.
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
            return Ok(Coins::new_empty());
        }

        let mut map = BTreeMap::new();
        for coin_str in s.split(',') {
            let Some((denom, amount_str)) = coin_str.split_once(':') else {
                return Err(StdError::parse_coins(format!(
                    "invalid coin `{coin_str}`: must be in the format {{denom}}:{{amount}}"
                )));
            };

            let Ok(amount) = Uint128::from_str(amount_str) else {
                return Err(StdError::parse_coins(format!(
                    "invalid amount `{amount_str}`"
                )));
            };

            if amount.is_zero() {
                return Err(StdError::parse_coins(format!(
                    "denom `{denom}` as zero amount"
                )));
            }

            if map.contains_key(denom) {
                return Err(StdError::parse_coins(format!("duplicate denom: {denom}")));
            }

            map.insert(denom.into(), amount);
        }

        Ok(Self(map))
    }
}

// create a new Coins instance from a vector of coins. the vector must not
// contain duplicate denoms or zero amounts.
impl TryFrom<Vec<Coin>> for Coins {
    type Error = StdError;

    fn try_from(vec: Vec<Coin>) -> Result<Self, Self::Error> {
        let mut map = BTreeMap::new();
        for coin in vec {
            if coin.amount.is_zero() {
                return Err(StdError::parse_coins(format!(
                    "denom `{}` as zero amount",
                    coin.denom
                )));
            }
            if map.insert(coin.denom, coin.amount).is_some() {
                return Err(StdError::parse_coins("duplicate denom found"));
            }
        }
        Ok(Self(map))
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

pub struct CoinsIter<'a>(btree_map::Iter<'a, String, Uint128>);

impl<'a> Iterator for CoinsIter<'a> {
    type Item = CoinRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0
            .next()
            .map(|(denom, amount)| CoinRef { denom, amount })
    }
}

pub struct CoinsIntoIter(btree_map::IntoIter<String, Uint128>);

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
        super::*,
        crate::{from_json_value, to_json_value, Json},
        serde_json::json,
    };

    fn mock_coins() -> Coins {
        Coins(
            [
                (String::from("uatom"), Uint128::new(123)),
                (String::from("umars"), Uint128::new(456)),
                (String::from("uosmo"), Uint128::new(789)),
            ]
            .into(),
        )
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
        assert_eq!(to_json_value(&mock_coins()).unwrap(), mock_coins_json());
    }

    #[test]
    fn deserializing_coins() {
        // valid string
        assert_eq!(
            from_json_value::<Coins>(mock_coins_json()).unwrap(),
            mock_coins()
        );

        // invalid json: contains zero amount
        let illegal_json = json!([
            {
                "denom": "uatom",
                "amount": "0",
            },
        ]);
        assert!(from_json_value::<Coins>(illegal_json).is_err());

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
        assert!(from_json_value::<Coins>(illegal_json).is_err());
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
}
