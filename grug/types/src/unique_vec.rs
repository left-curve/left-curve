use {
    crate::{Inner, Lengthy, StdError, StdResult},
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Serialize, de},
    std::{collections::HashSet, hash::Hash, io, vec},
};

/// A wrapper over a vector that guarantees that no element appears twice.
///
/// This is useful if you want to ensure a collection of items is unique, and
/// also _ordered_ (in which case `BTreeSet` isn't suitable).
#[derive(Serialize, BorshSerialize, Default, Debug, Clone, PartialEq, Eq)]
pub struct UniqueVec<T>(Vec<T>)
where
    T: Eq + Hash;

impl<T> UniqueVec<T>
where
    T: Eq + Hash,
{
    // Here we collect the elements into a set, and check whether the set has
    // the same length as the vector.
    // Different trait bounds are required using HashSet or BTreeSet.
    // HashSet has faster insertion and lookup, while BTreeSet has faster
    // comparison if `T` is a simple number type such as `u32`.
    // Overall, we choose to use a HashSet here.
    pub fn new(inner: Vec<T>) -> StdResult<Self> {
        if inner.iter().collect::<HashSet<_>>().len() != inner.len() {
            return Err(StdError::duplicate_data::<T>());
        }

        Ok(Self(inner))
    }

    pub fn new_unchecked(inner: Vec<T>) -> Self {
        Self(inner)
    }

    /// Check if the item is already in the vector, and if not, push it.
    pub fn try_push(&mut self, item: T) -> StdResult<()> {
        if self.0.contains(&item) {
            return Err(StdError::duplicate_data::<T>());
        }

        self.0.push(item);

        Ok(())
    }
}

impl<T> Inner for UniqueVec<T>
where
    T: Eq + Hash,
{
    type U = Vec<T>;

    fn inner(&self) -> &Self::U {
        &self.0
    }

    fn into_inner(self) -> Self::U {
        self.0
    }
}

impl<T> Lengthy for UniqueVec<T>
where
    T: Eq + Hash,
{
    fn length(&self) -> usize {
        self.0.len()
    }
}

impl<T> IntoIterator for UniqueVec<T>
where
    T: Eq + Hash,
{
    type IntoIter = vec::IntoIter<T>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<T> TryFrom<Vec<T>> for UniqueVec<T>
where
    T: Eq + Hash,
{
    type Error = StdError;

    fn try_from(vector: Vec<T>) -> StdResult<Self> {
        Self::new(vector)
    }
}

impl<'de, T> de::Deserialize<'de> for UniqueVec<T>
where
    T: Eq + Hash + de::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        <Vec<T> as de::Deserialize>::deserialize(deserializer)?
            .try_into()
            .map_err(de::Error::custom)
    }
}

impl<T> BorshDeserialize for UniqueVec<T>
where
    T: Eq + Hash + BorshDeserialize,
{
    fn deserialize_reader<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        <Vec<T> as BorshDeserialize>::deserialize_reader(reader)?
            .try_into()
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use crate::{JsonDeExt, ResultExt, UniqueVec};

    #[test]
    fn deserializing_unique_vec() {
        b"[1, 2, 3, 4, 5]"
            .deserialize_json::<UniqueVec<u32>>()
            .should_succeed_and_equal(UniqueVec::new_unchecked(vec![1, 2, 3, 4, 5]));

        b"[1, 2, 3, 1, 5]"
            .deserialize_json::<UniqueVec<u32>>()
            .should_fail_with_error("duplicate data found!");
    }

    #[test]
    fn unique_vec_try_push() {
        let mut unique_vec = UniqueVec::new_unchecked(vec![1, 2, 3]);
        unique_vec.try_push(4).should_succeed();
        unique_vec
            .try_push(3)
            .should_fail_with_error("duplicate data found!");
    }
}
