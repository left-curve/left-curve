use std::{
    fmt::{self, Display},
    ops::Deref,
};

// #[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
#[grug::derive(Borsh)]
#[derive(Copy, Default, Ord, PartialOrd)]
pub struct Height(u64);

impl Height {
    pub fn new(height: u64) -> Self {
        Self(height)
    }
}

impl Deref for Height {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for Height {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl malachitebft_core_types::Height for Height {
    const INITIAL: Self = Self(1);
    const ZERO: Self = Self(0);

    fn increment_by(&self, n: u64) -> Self {
        Self(self.0 + n)
    }

    fn decrement_by(&self, n: u64) -> Option<Self> {
        self.0.checked_sub(n).map(Self)
    }

    fn as_u64(&self) -> u64 {
        self.0
    }
}
