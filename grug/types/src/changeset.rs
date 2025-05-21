use {
    crate::{StdError, StdResult},
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{
        Deserialize, Serialize,
        de::{self, Error},
    },
    std::{
        collections::{BTreeMap, BTreeSet},
        io,
    },
};

/// A set of changes applicable to a map-like data structure.
///
/// This struct implements a custom deserialization method that ensures there's
/// no intersection between the keys to be added and those to be removed.
#[derive(Serialize, BorshSerialize, Debug, Clone, PartialEq, Eq)]
pub struct ChangeSet<K, V> {
    /// For adding new key-value pairs, or updating the values associated with
    /// existing keys.
    add: BTreeMap<K, V>,
    /// For removing existing keys.
    remove: BTreeSet<K>,
}

impl<K, V> ChangeSet<K, V>
where
    K: Ord,
{
    /// Create a new `ChangeSet`.
    /// Error if `add` and `remove` have an intersection.
    pub fn new(add: BTreeMap<K, V>, remove: BTreeSet<K>) -> StdResult<Self> {
        if add.keys().any(|k| remove.contains(k)) {
            return Err(StdError::InvalidChangeSet);
        }

        Ok(Self { add, remove })
    }

    /// Create a new `ChangeSet` but without checking for intersection.
    pub fn new_unchecked(add: BTreeMap<K, V>, remove: BTreeSet<K>) -> Self {
        Self { add, remove }
    }

    /// Return the `add` map as a reference.
    pub fn add(&self) -> &BTreeMap<K, V> {
        &self.add
    }

    /// Consume self, return the `add` map by value.
    pub fn into_add(self) -> BTreeMap<K, V> {
        self.add
    }

    /// Return the `remove` set as a reference.
    pub fn remove(&self) -> &BTreeSet<K> {
        &self.remove
    }

    /// Consume self, return the `remove` set by value.
    pub fn into_remove(self) -> BTreeSet<K> {
        self.remove
    }
}

#[derive(Deserialize, BorshDeserialize)]
struct UncheckedChangeSet<K, V>
where
    K: Ord,
    V: Ord,
{
    add: BTreeMap<K, V>,
    remove: BTreeSet<K>,
}

impl<'de, K, V> de::Deserialize<'de> for ChangeSet<K, V>
where
    K: Ord + Deserialize<'de>,
    V: Ord + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let unchecked: UncheckedChangeSet<K, V> = Deserialize::deserialize(deserializer)?;

        ChangeSet::new(unchecked.add, unchecked.remove).map_err(D::Error::custom)
    }
}

impl<K, V> BorshDeserialize for ChangeSet<K, V>
where
    K: Ord + BorshDeserialize,
    V: Ord + BorshDeserialize,
{
    fn deserialize_reader<R>(reader: &mut R) -> io::Result<Self>
    where
        R: io::Read,
    {
        let unchecked: UncheckedChangeSet<K, V> = BorshDeserialize::deserialize_reader(reader)?;

        ChangeSet::new(unchecked.add, unchecked.remove).map_err(io::Error::other)
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use crate::{ChangeSet, JsonDeExt, json};

    #[test]
    fn deserializing_changeset() {
        // No intersection
        assert!(
            json!({
                "add": {
                    "a": 1,
                    "b": 2,
                    "c": 3,
                },
                "remove": ["d", "e", "f"],
            })
            .deserialize_json::<ChangeSet<String, usize>>()
            .is_ok()
        );

        // Has non-empty intersection
        assert!(
            json!({
                "add": {
                    "a": 1,
                    "b": 2,
                    "c": 3,
                },
                "remove": ["c", "d", "e"],
            })
            .deserialize_json::<ChangeSet<String, usize>>()
            .is_err()
        );
    }
}
