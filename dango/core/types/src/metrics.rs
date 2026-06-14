pub struct MetricsIter<I, L>
where
    I: Iterator,
{
    iter: I,
    #[cfg_attr(not(feature = "metrics"), allow(dead_code))]
    name: &'static str,
    #[cfg_attr(not(feature = "metrics"), allow(dead_code))]
    labels: L,
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

pub trait MetricsIterExt: Sized + Iterator {
    fn with_metrics<L>(self, name: &'static str, labels: L) -> MetricsIter<Self, L>;
}

impl<T> MetricsIterExt for T
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
