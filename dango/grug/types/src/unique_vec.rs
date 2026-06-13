use {
    crate::{Checker, Lengthy, Predicate, StdError, StdResult},
    std::{collections::HashSet, hash::Hash, vec},
};

/// Checker that rejects duplicate elements in a vector.
pub struct IsUnique;

impl<T> Checker<Vec<T>> for IsUnique
where
    T: Eq + Hash,
{
    fn check(value: &Vec<T>) -> StdResult<()> {
        if value.iter().collect::<HashSet<_>>().len() != value.len() {
            return Err(StdError::duplicate_data::<T>());
        }

        Ok(())
    }
}

/// A wrapper over a vector that guarantees that no element appears twice.
///
/// This is useful if you want to ensure a collection of items is unique, and
/// also _ordered_ (in which case `BTreeSet` isn't suitable).
pub type UniqueVec<T> = Predicate<Vec<T>, IsUnique>;

impl<T> UniqueVec<T>
where
    T: Eq + Hash,
{
    /// Check if the item is already in the vector, and if not, push it.
    pub fn try_push(&mut self, item: T) -> StdResult<()> {
        if self.value.contains(&item) {
            return Err(StdError::duplicate_data::<T>());
        }

        self.value.push(item);

        Ok(())
    }
}

impl<T> Lengthy for UniqueVec<T>
where
    T: Eq + Hash,
{
    fn length(&self) -> usize {
        self.value.len()
    }
}

impl<T> IntoIterator for UniqueVec<T>
where
    T: Eq + Hash,
{
    type IntoIter = vec::IntoIter<T>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        self.value.into_iter()
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
