use {
    crate::{StdError, StdResult, Uint128},
    serde::{de, ser, ser::SerializeSeq, Deserialize, Serialize},
    std::{collections::{BTreeMap, btree_map}, fmt},
};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Coin {
    pub denom:  String,
    pub amount: Uint128,
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
    pub denom:  &'a String,
    pub amount: &'a Uint128,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Coins(BTreeMap<String, Uint128>);

impl Coins {
    pub fn empty() -> Self {
        Self(BTreeMap::new())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Return whether there is a non-zero amount of the given denom.
    pub fn has(&self, denom: &str) -> bool {
        self.0.get(denom).is_some()
    }

    /// Get the amount of the given denom.
    /// Note, if the denom does not exist, zero is returned.
    pub fn amount_of(&self, denom: &str) -> Uint128 {
        self.0.get(denom).copied().unwrap_or_else(Uint128::zero)
    }

    /// Increase the amount of a denom by the given amount. If the denom doesn't
    /// exist, a new record is created.
    // TODO: replace anyhow::Error with StdError.
    pub fn increase_amount(&mut self, denom: &str, by: Uint128) -> anyhow::Result<()> {
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
    // TODO: replace anyhow::Error with StdError.
    pub fn decrease_amount(&mut self, denom: &str, by: Uint128) -> StdResult<()> {
        let Some(amount) = self.0.get_mut(denom) else {
            return Err(StdError::DenomNotFound { denom: denom.into() });
        };

        // TODO
        *amount = amount.checked_sub(by).unwrap();

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

impl TryFrom<Vec<Coin>> for Coins {
    type Error = StdError;

    fn try_from(coins: Vec<Coin>) -> Result<Self, Self::Error> {
        let mut map = BTreeMap::new();
        for coin in coins {
            if map.insert(coin.denom, coin.amount).is_some() {
                return Err(StdError::DuplicateDenom);
            }
        }
        Ok(Self(map))
    }
}

impl From<Coins> for Vec<Coin> {
    fn from(coins: Coins) -> Self {
        coins.into_iter().collect()
    }
}

impl<'a> IntoIterator for &'a Coins {
    type Item = CoinRef<'a>;
    type IntoIter = CoinsIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        CoinsIter(self.0.iter())
    }
}

impl IntoIterator for Coins {
    type Item = Coin;
    type IntoIter = CoinsIntoIter;

    fn into_iter(self) -> Self::IntoIter {
        CoinsIntoIter(self.0.into_iter())
    }
}

pub struct CoinsIter<'a>(btree_map::Iter<'a, String, Uint128>);

impl<'a> Iterator for CoinsIter<'a> {
    type Item = CoinRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(denom, amount)| CoinRef { denom, amount })
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
        for coin in self {
            write!(f, "{}:{}", coin.denom, coin.amount)?;
        }
        Ok(())
    }
}

impl fmt::Debug for Coins {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Coins(")?;
        for coin in self {
            write!(f, "{}:{}", coin.denom, coin.amount)?;
        }
        f.write_str(")")
    }
}

// although we store coins in a BTreeMap, cw-serde-json doesn't support
// serializing maps, so we have to serialize it to an array.
impl ser::Serialize for Coins {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for (denom, amount) in &self.0 {
            seq.serialize_element(&CoinRef { denom, amount })?;
        }
        seq.end()
    }
}

impl<'de> de::Deserialize<'de> for Coins {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(CoinsVisitor)
    }
}

struct CoinsVisitor;

impl<'de> de::Visitor<'de> for CoinsVisitor {
    type Value = Coins;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("A sequence of coins")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let mut map = BTreeMap::new();
        // note: we ensure that there is no duplicate denom or zero amounts.
        // unlike in cosmos-sdk, we don't ensure that denoms are sorted.
        while let Some(Coin { denom, amount }) = seq.next_element()? {
            if amount.is_zero() {
                return Err(de::Error::custom("Coin amount is zero"));
            }
            if map.insert(denom, amount).is_some() {
                return Err(de::Error::custom("Duplicate denom found"));
            }
        }
        Ok(Coins(map))
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{from_json, to_json},
    };

    const MOCK_COINS_STR: &[u8] = br#"[{"denom":"uatom","amount":"123"},{"denom":"umars","amount":"456"},{"denom":"uosmo","amount":"789"}]"#;

    fn mock_coins() -> Coins {
        Coins([
            (String::from("uatom"), Uint128::new(123)),
            (String::from("umars"), Uint128::new(456)),
            (String::from("uosmo"), Uint128::new(789)),
        ]
        .into())
    }

    #[test]
    fn serializing_coins() {
        assert_eq!(to_json(&mock_coins()).unwrap().as_ref(), MOCK_COINS_STR);
    }

    #[test]
    fn deserializing_coins() {
        // valid string
        assert_eq!(from_json::<Coins>(MOCK_COINS_STR).unwrap(), mock_coins());

        // invalid string: contains zero amount
        let s = br#"[{"denom":"uatom","amount":"0"}]"#;
        assert!(from_json::<Coins>(s).is_err());

        // invalid string: contains duplicate
        let s = br#"[{"denom":"uatom","amount":"123"},{"denom":"uatom","amount":"456"}]"#;
        assert!(from_json::<Coins>(s).is_err());
    }
}
