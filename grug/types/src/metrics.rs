pub struct MetricsIter<I, L>
where
    I: Iterator,
    for<'a> &'a L: metrics::IntoLabels,
{
    iter: I,
    name: &'static str,
    labels: L,
}

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

pub trait MetricsIterExt: Sized + Iterator {
    fn with_metrics<L>(self, name: &'static str, labels: L) -> MetricsIter<Self, L>
    where
        for<'a> &'a L: metrics::IntoLabels;
}

impl<T> MetricsIterExt for T
where
    T: Iterator,
{
    fn with_metrics<L>(self, name: &'static str, labels: L) -> MetricsIter<Self, L>
    where
        for<'a> &'a L: metrics::IntoLabels,
    {
        MetricsIter {
            iter: self,
            name,
            labels,
        }
    }
}
