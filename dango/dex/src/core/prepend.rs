pub struct Prepend<I, T>
where
    I: Iterator<Item = T>,
{
    inner: I,
    item: Option<T>,
}

impl<I, T> Prepend<I, T>
where
    I: Iterator<Item = T>,
{
    pub fn prepend(&mut self, item: T) {
        assert!(self.item.is_none(), "an item is already prepended");
        self.item = Some(item);
    }
}

impl<I, T> Iterator for Prepend<I, T>
where
    I: Iterator<Item = T>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        // If there is an item prepended, return it first.
        if let Some(item) = self.item.take() {
            return Some(item);
        }

        // Otherwise, advance the iterator normally.
        self.inner.next()
    }
}

pub trait Prependable<T>: Sized + Iterator<Item = T> {
    fn prependable(self) -> Prepend<Self, T> {
        Prepend {
            inner: self,
            item: None,
        }
    }
}

impl<I, T> Prependable<T> for I where I: Iterator<Item = T> {}
