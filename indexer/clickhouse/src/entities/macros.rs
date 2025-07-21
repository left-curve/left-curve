/// Sadly this is not working to make code DRY, the `ComplexObject` doesn't pick
/// methods from the `impl` block. But it should work.
#[cfg(feature = "async-graphql")]
#[macro_export]
macro_rules! bigdecimal_method {
    ($name:ident, $scale:expr) => {
        #[allow(dead_code)]
        async fn $name(&self) -> BigDecimal {
            let inner_value = self.$name.inner();
            let bigint = BigInt::from(*inner_value);
            BigDecimal::new(bigint, $scale).normalized()
        }
    };
}
