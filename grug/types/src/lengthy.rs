use {
    crate::Binary,
    std::collections::{BTreeMap, BTreeSet},
};

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
