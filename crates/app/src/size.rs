#[derive(Copy, Clone, Debug)]
pub struct Size(pub(crate) usize);

impl Size {
    /// Creates a size of `n`
    pub const fn new(n: usize) -> Self {
        Size(n)
    }

    /// Creates a size of `n` kilo
    pub const fn kilo(n: usize) -> Self {
        Size(n * 1000)
    }

    /// Creates a size of `n` kibi
    pub const fn kibi(n: usize) -> Self {
        Size(n * 1024)
    }

    /// Creates a size of `n` mega
    pub const fn mega(n: usize) -> Self {
        Size(n * 1000 * 1000)
    }

    /// Creates a size of `n` mebi
    pub const fn mebi(n: usize) -> Self {
        Size(n * 1024 * 1024)
    }

    /// Creates a size of `n` giga
    pub const fn giga(n: usize) -> Self {
        Size(n * 1000 * 1000 * 1000)
    }

    /// Creates a size of `n` gibi
    pub const fn gibi(n: usize) -> Self {
        Size(n * 1024 * 1024 * 1024)
    }
}

impl From<Size> for usize {
    fn from(value: Size) -> Self {
        value.0
    }
}
