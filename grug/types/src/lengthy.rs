use {
    crate::Binary,
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

impl Lengthy for Vec<u8> {
    fn length(&self) -> usize {
        self.len()
    }
}

impl<const LEN: usize> Lengthy for [u8; LEN] {
    fn length(&self) -> usize {
        self.len()
    }
}

impl<K, V> Lengthy for BTreeMap<K, V> {
    fn length(&self) -> usize {
        self.len()
    }
}

impl<K> Lengthy for BTreeSet<K> {
    fn length(&self) -> usize {
        self.len()
    }
}
