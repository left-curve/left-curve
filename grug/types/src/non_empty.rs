use {
    crate::{StdError, StdResult},
    borsh::{BorshDeserialize, BorshSerialize},
    grug_math::Inner,
    serde::{de, Serialize},
    std::{
        fmt::{self, Display},
        io,
        ops::Deref,
    },
};

#[derive(Serialize, BorshSerialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct NonEmpty<T>(T)
where
    for<'a> &'a T: IntoIterator;

impl<T> NonEmpty<T>
where
    for<'a> &'a T: IntoIterator,
{
    pub fn new(inner: T) -> StdResult<Self> {
        if inner.into_iter().next().is_none() {
            return Err(StdError::empty_value::<T>());
        }

        Ok(Self(inner))
    }

    pub fn new_unchecked(inner: T) -> Self {
        Self(inner)
    }
}

impl<T> Inner for NonEmpty<T>
where
    for<'a> &'a T: IntoIterator,
{
    type U = T;

    fn inner(&self) -> &Self::U {
        &self.0
    }

    fn into_inner(self) -> Self::U {
        self.0
    }
}

impl<T> AsRef<T> for NonEmpty<T>
where
    for<'a> &'a T: IntoIterator,
{
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T> Deref for NonEmpty<T>
where
    for<'a> &'a T: IntoIterator,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> Display for NonEmpty<T>
where
    for<'a> &'a T: IntoIterator,
    T: Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<'de, T> de::Deserialize<'de> for NonEmpty<T>
where
    T: de::Deserialize<'de>,
    for<'a> &'a T: IntoIterator,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let inner = T::deserialize(deserializer)?;

        NonEmpty::new(inner).map_err(de::Error::custom)
    }
}

impl<T> BorshDeserialize for NonEmpty<T>
where
    T: BorshDeserialize,
    for<'a> &'a T: IntoIterator,
{
    fn deserialize_reader<R>(reader: &mut R) -> io::Result<Self>
    where
        R: io::Read,
    {
        let inner = BorshDeserialize::deserialize_reader(reader)?;

        NonEmpty::new(inner).map_err(|err| io::Error::new(io::ErrorKind::Other, err))
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        crate::{btree_map, btree_set, JsonDeExt, NonEmpty, ResultExt},
        std::collections::{BTreeMap, BTreeSet},
    };

    #[test]
    fn deserializing_non_empty() {
        // Non-empty vector
        "[1, 2, 3]"
            .deserialize_json::<NonEmpty<Vec<u32>>>()
            .should_succeed_and_equal(NonEmpty::new_unchecked(vec![1, 2, 3]));

        // Empty vector
        "[]".deserialize_json::<NonEmpty<Vec<u32>>>()
            .should_fail_with_error("expecting a non-empty value");

        // Non-empty B-tree set
        "[1, 2, 3]"
            .deserialize_json::<NonEmpty<BTreeSet<u32>>>()
            .should_succeed_and_equal(NonEmpty::new_unchecked(btree_set! { 1, 2, 3 }));

        // Empty B-tree set
        "[]".deserialize_json::<NonEmpty<BTreeSet<u32>>>()
            .should_fail_with_error("expecting a non-empty value");

        // Non-empty B-tree map
        "{\"a\":1,\"b\":2,\"c\":3}"
            .deserialize_json::<NonEmpty<BTreeMap<String, u32>>>()
            .should_succeed_and_equal(NonEmpty::new_unchecked(btree_map! {
                "a".to_string() => 1,
                "b".to_string() => 2,
                "c".to_string() => 3,
            }));

        // Empty B-tree map
        "{}".deserialize_json::<NonEmpty<BTreeMap<String, u32>>>()
            .should_fail_with_error("expecting a non-empty value");
    }
}
