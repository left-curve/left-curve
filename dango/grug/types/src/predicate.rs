use {
    crate::{Inner, StdResult},
    borsh::{BorshDeserialize, BorshSerialize},
    serde::de::{self, Error},
    std::{
        fmt::{self, Debug, Display},
        hash::Hash,
        io,
        marker::PhantomData,
        ops::Deref,
    },
};

/// A static validation function for a value of type `T`.
pub trait Checker<T> {
    fn check(value: &T) -> StdResult<()>;
}

/// A wrapper that validates a value of type `T` using a checker `C`.
///
/// - `new` runs `C::check` on the value.
/// - `new_unchecked` skips the check.
/// - Serde deserialization runs `C::check`; borsh deserialization skips it
///   (borsh is only used for contract-internal storage, already validated at
///   write time).
pub struct Predicate<T, C>
where
    C: Checker<T>,
{
    pub(crate) value: T,
    _checker: PhantomData<C>,
}

impl<T, C> Predicate<T, C>
where
    C: Checker<T>,
{
    pub fn new(value: T) -> StdResult<Self> {
        C::check(&value)?;

        Ok(Self {
            value,
            _checker: PhantomData,
        })
    }

    pub const fn new_unchecked(value: T) -> Self {
        Self {
            value,
            _checker: PhantomData,
        }
    }
}

// ----------------------------- blanket impls ---------------------------------

impl<T, C> Inner for Predicate<T, C>
where
    C: Checker<T>,
{
    type U = T;

    fn inner(&self) -> &Self::U {
        &self.value
    }

    fn into_inner(self) -> Self::U {
        self.value
    }
}

impl<T, C> AsRef<T> for Predicate<T, C>
where
    C: Checker<T>,
{
    fn as_ref(&self) -> &T {
        &self.value
    }
}

impl<T, C> Deref for Predicate<T, C>
where
    C: Checker<T>,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T, C> Display for Predicate<T, C>
where
    T: Display,
    C: Checker<T>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

// --- serde: validates on deserialize ---

impl<T, C> serde::Serialize for Predicate<T, C>
where
    T: serde::Serialize,
    C: Checker<T>,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.value.serialize(serializer)
    }
}

impl<'de, T, C> de::Deserialize<'de> for Predicate<T, C>
where
    T: de::Deserialize<'de>,
    C: Checker<T>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let value = T::deserialize(deserializer)?;

        Self::new(value).map_err(D::Error::custom)
    }
}

// --- borsh: skips validation on deserialize ---

impl<T, C> BorshSerialize for Predicate<T, C>
where
    T: BorshSerialize,
    C: Checker<T>,
{
    fn serialize<W>(&self, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        self.value.serialize(writer)
    }
}

impl<T, C> BorshDeserialize for Predicate<T, C>
where
    T: BorshDeserialize,
    C: Checker<T>,
{
    fn deserialize_reader<R>(reader: &mut R) -> io::Result<Self>
    where
        R: io::Read,
    {
        let value = T::deserialize_reader(reader)?;

        Ok(Self::new_unchecked(value))
    }
}

// --- manual derive impls that only bound T, not C ---

impl<T, C> Debug for Predicate<T, C>
where
    T: Debug,
    C: Checker<T>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Predicate").field(&self.value).finish()
    }
}

impl<T, C> Clone for Predicate<T, C>
where
    T: Clone,
    C: Checker<T>,
{
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            _checker: PhantomData,
        }
    }
}

impl<T, C> Copy for Predicate<T, C>
where
    T: Copy,
    C: Checker<T>,
{
}

impl<T, C> PartialEq for Predicate<T, C>
where
    T: PartialEq,
    C: Checker<T>,
{
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T, C> Eq for Predicate<T, C>
where
    T: Eq,
    C: Checker<T>,
{
}

impl<T, C> PartialOrd for Predicate<T, C>
where
    T: PartialOrd,
    C: Checker<T>,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

impl<T, C> Ord for Predicate<T, C>
where
    T: Ord,
    C: Checker<T>,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value.cmp(&other.value)
    }
}

impl<T, C> Hash for Predicate<T, C>
where
    T: Hash,
    C: Checker<T>,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl<T, C> Default for Predicate<T, C>
where
    T: Default,
    C: Checker<T>,
{
    fn default() -> Self {
        Self {
            value: T::default(),
            _checker: PhantomData,
        }
    }
}
