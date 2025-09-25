use {grug::Denom, std::time::Instant};

/// A wrapper over an iterator, that measure the time consumption of each `.next()`
/// call and emit the data using the metrics API.
pub struct TimedIterator<I> {
    iter: I,
    label: &'static str,
    base_denom: Denom,
    quote_denom: Denom,
}

impl<I> TimedIterator<I> {
    pub fn new(iter: I, label: &'static str, base_denom: Denom, quote_denom: Denom) -> Self {
        Self {
            iter,
            label,
            base_denom,
            quote_denom,
        }
    }
}

impl<I> Iterator for TimedIterator<I>
where
    I: Iterator,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let now = Instant::now();
        let item = self.iter.next();

        metrics::histogram!(
            self.label,
            "base_denom" => self.base_denom.to_string(),
            "quote_denom" => self.quote_denom.to_string(),
        )
        .record(now.elapsed().as_secs_f64());

        item
    }
}
