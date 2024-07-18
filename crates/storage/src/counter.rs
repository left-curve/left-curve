use {
    crate::{Borsh, Codec, Item},
    grug_types::{StdResult, Storage},
};

// ----------------------------------- trait -----------------------------------

pub trait Increment {
    /// Return the initial value where incrementing should start from.
    const ZERO: Self;

    /// Return the number that is one unit bigger than self.
    fn increment(&self) -> Self;
}

macro_rules! impl_increment {
    ($($t:ty),+ $(,)?) => {
        $(impl Increment for $t {
            const ZERO: Self = 0;

            fn increment(&self) -> Self {
                *self + 1
            }
        })*
    }
}

impl_increment!(u8, u16, u32, u64, u128);

// TODO: implement for Uint64/128/256

// ------------------------------ storage object -------------------------------

/// An abstraction over `Item`. Stores a single number that is monotonically
/// incremented one unit at a time.
pub struct Counter<'a, T, C = Borsh>
where
    C: Codec<T>,
{
    item: Item<'a, T, C>,
}

impl<'a, T, C> Counter<'a, T, C>
where
    C: Codec<T>,
{
    pub const fn new(storage_key: &'a str) -> Self {
        Self {
            item: Item::new(storage_key),
        }
    }
}

impl<'a, T, C> Counter<'a, T, C>
where
    T: Increment,
    C: Codec<T>,
{
    /// Initialize the incrementor to the zero value.
    ///
    /// This is typically done during contract instantiation.
    pub fn initialize(&self, storage: &mut dyn Storage) -> StdResult<()> {
        self.item.save(storage, &T::ZERO)
    }

    /// Load the current counter value.
    ///
    /// Error if the counter has not been initialized.
    pub fn load(&self, storage: &dyn Storage) -> StdResult<T> {
        self.item.load(storage)
    }

    /// Increment the value stored by one unit; return the value _after_
    /// incrementing. If no value is stored, set it to zero and return zero.
    pub fn increment(&self, storage: &mut dyn Storage) -> StdResult<T> {
        let new_value = match self.item.may_load(storage)? {
            Some(old_value) => old_value.increment(),
            None => T::ZERO,
        };

        self.item.save(storage, &new_value)?;

        Ok(new_value)
    }
}
