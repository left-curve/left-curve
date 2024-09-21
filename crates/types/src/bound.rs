use {
    crate::{StdError, StdResult},
    std::{marker::PhantomData, ops::Bound},
};

pub trait Bounds<T> {
    const MIN: Bound<T>;
    const MAX: Bound<T>;
}

#[derive(Debug)]
pub struct Bounded<T, B>(T, PhantomData<B>);

impl<T, B> Bounded<T, B>
where
    T: PartialOrd + ToString,
    B: Bounds<T>,
{
    pub fn new(value: T) -> StdResult<Self> {
        match B::MIN {
            Bound::Included(bound) if value < bound => {
                return Err(StdError::out_of_range(value, "<", bound));
            },
            Bound::Excluded(bound) if value <= bound => {
                return Err(StdError::out_of_range(value, "<=", bound));
            },
            _ => (),
        }

        match B::MAX {
            Bound::Included(bound) if value > bound => {
                return Err(StdError::out_of_range(value, ">", bound));
            },
            Bound::Excluded(bound) if value >= bound => {
                return Err(StdError::out_of_range(value, ">=", bound));
            },
            _ => (),
        }

        Ok(Self(value, PhantomData))
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

// Note: tests are found in crates/std/tests/bounded.rs
