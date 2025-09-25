#[macro_export]
macro_rules! metric {
    ({ $($tt:tt)* }) => {
        #[cfg(feature = "metrics")]
        { $($tt)* }
    };
    ($stmt:stmt) => {
        #[cfg(feature = "metrics")]
        $stmt;
    };
}

#[cfg(feature = "metrics")]
mod _metrics {

    use std::time::Instant;

    pub use metrics::*;

    pub struct TimerGuard {
        label: &'static str,
        start: Instant,
    }

    impl TimerGuard {
        pub fn now(label: &'static str) -> Self {
            Self {
                label,
                start: Instant::now(),
            }
        }

        pub fn end(self) {
            metrics::histogram!(self.label).record(self.start.elapsed().as_secs_f64());
        }

        pub fn end_with<P>(self, params: P)
        where
            P: IntoLabels,
        {
            metrics::histogram!(self.label, params).record(self.start.elapsed().as_secs_f64());
        }
    }
}

#[cfg(feature = "metrics")]
pub use _metrics::*;
