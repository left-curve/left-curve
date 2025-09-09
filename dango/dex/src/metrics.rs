#[macro_export]
macro_rules! metric {
    ( $($stmt:stmt)* ) => {
        $( #[cfg(feature = "metrics")] $stmt)*
    };
}
