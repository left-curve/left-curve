/// Given that T == U is implemented, also implement &T == U and T == &U.
/// Useful in creating math types.
///
/// Copied from CosmWasm:
/// https://github.com/CosmWasm/cosmwasm/blob/v1.5.3/packages/std/src/forward_ref.rs
#[macro_export]
macro_rules! forward_ref_partial_eq {
    ($t:ty, $u:ty) => {
        // &T == U
        impl<'a> PartialEq<$u> for &'a $t {
            #[inline]
            fn eq(&self, rhs: &$u) -> bool {
                **self == *rhs
            }
        }

        // T == &U
        impl PartialEq<&$u> for $t {
            #[inline]
            fn eq(&self, rhs: &&$u) -> bool {
                *self == **rhs
            }
        }
    }
}
