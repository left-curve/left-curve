use {
    crate::{Borsh, Codec, Item},
    grug_types::{Number, StdResult, Storage},
};

/// An abstraction over `Item`. Stores a single number that is monotonically
/// incremented one unit at a time.
pub struct Counter<'a, T, C = Borsh>
where
    T: Number + Copy,
    C: Codec<T>,
{
    item: Item<'a, T, C>,
    base: T,
    step: T,
}

impl<'a, T, C> Counter<'a, T, C>
where
    T: Number + Copy,
    C: Codec<T>,
{
    pub const fn new(storage_key: &'a str, base: T, step: T) -> Self {
        Self {
            item: Item::new(storage_key),
            base,
            step,
        }
    }

    /// Load the current counter value.
    pub fn current(&self, storage: &dyn Storage) -> StdResult<T> {
        self.item
            .may_load(storage)
            .map(|maybe_value| maybe_value.unwrap_or(self.base))
    }

    /// Increment the value stored by one unit; return the values before and
    /// after incrementing.
    pub fn increment(&self, storage: &mut dyn Storage) -> StdResult<(T, T)> {
        let old_value = self.current(storage)?;
        let new_value = old_value.checked_add(self.step)?;

        self.item.save(storage, &new_value)?;

        Ok((old_value, new_value))
    }
}
