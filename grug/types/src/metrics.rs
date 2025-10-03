pub struct MetricsIter<I, L> {
    iter: I,
    pub name: &'static str,
    pub labels: L,
}

#[cfg(not(feature = "metrics"))]
impl<I, L> Iterator for MetricsIter<I, L>
where
    I: Iterator,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

#[cfg(feature = "metrics")]
impl<I, L> Iterator for MetricsIter<I, L>
where
    I: Iterator,
    for<'a> &'a L: metrics::IntoLabels,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let duration = std::time::Instant::now();

        let item = self.iter.next();

        metrics::histogram!(self.name, &self.labels).record(duration.elapsed().as_secs_f64());

        item
    }
}

pub trait MetricsIterExt<'a>: Sized {
    fn with_metrics<L>(self, name: &'static str, labels: L) -> MetricsIter<Self, L>;
}

impl<'a, T> MetricsIterExt<'a> for T
where
    T: Iterator,
{
    fn with_metrics<L>(self, name: &'static str, labels: L) -> MetricsIter<Self, L> {
        MetricsIter {
            iter: self,
            name,
            labels,
        }
    }
}
