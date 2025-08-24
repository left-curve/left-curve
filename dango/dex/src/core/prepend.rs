/// A wrapper over an iterator that allows to insert an item to the beginning of it.
pub struct Prepend<I>
where
    I: Iterator,
{
    iter: I,
    prepended: Option<I::Item>,
}

impl<I> Prepend<I>
where
    I: Iterator,
{
    pub fn new(iter: I, prepended: Option<I::Item>) -> Self {
        Self { iter, prepended }
    }
}

impl<I> Iterator for Prepend<I>
where
    I: Iterator,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(item) = self.prepended.take() {
            Some(item)
        } else {
            self.iter.next()
        }
    }
}
