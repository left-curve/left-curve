use {
    crate::{Binary, Coins},
    std::collections::{BTreeMap, BTreeSet},
};

/// Describes a value that can be measured in length.
///
/// We have to create this trait because Rust standard library doesn't have a
/// built-in trait for the `.len()` method.
pub trait Lengthy {
    fn length(&self) -> usize;
}

impl Lengthy for Binary {
    fn length(&self) -> usize {
        self.len()
    }
}

impl Lengthy for String {
    fn length(&self) -> usize {
        self.len()
    }
}

impl<T, const LEN: usize> Lengthy for [T; LEN] {
    fn length(&self) -> usize {
        LEN
    }
}

impl<T> Lengthy for Vec<T> {
    fn length(&self) -> usize {
        self.len()
    }
}

impl<T> Lengthy for BTreeSet<T> {
    fn length(&self) -> usize {
        self.len()
    }
}

impl<K, V> Lengthy for BTreeMap<K, V> {
    fn length(&self) -> usize {
        self.len()
    }
}

impl Lengthy for Coins {
    fn length(&self) -> usize {
        self.len()
    }
}
