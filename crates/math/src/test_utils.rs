use std::fmt::Debug;

/// `derive_type`
///
/// Allow compiler to derive the type of a variable,
/// which is necessary for the test functions.
pub(crate) fn dt<T>(_: T, _: T) {}

/// `derive_types`
///
///  Allow compiler to derive the types of multiple variables
#[macro_export(local_inner_macros)]
macro_rules! dts{
        ($u: expr, $($p:expr),* ) =>
         {
            $(dt($u, $p);)*
         }
    }

/// `built_type`
///
///  Allow compiler to derive the type of a variable, and return right.
pub(crate) fn bt<T>(_: T, ret: T) -> T {
    ret
}

/// Combines `assert_eq` and `derive_type` to derive the type and assert
pub(crate) fn smart_assert<T: Debug + PartialEq>(left: T, right: T) {
    assert_eq!(left, right);
}

// ------------------------------------ int ------------------------------------

/// Macro for unit tests for Int.
/// Is not possible to use [`test_case::test_case`] because the arguments types can are different.
/// Also `Int<U>` is different for each test case.
///
/// The macro set as first parameter of the callback function `Int::ZERO`, so the compiler can derive the type
/// (see [`derive_type`], [`derive_types`] and [`smart_assert`] ).
#[macro_export(local_inner_macros)]
macro_rules! int_test {
        // No Args
        (
            $name:ident,
            NoArgs
            $(attrs = $(#[$meta:meta])*)?
            => $test_fn:expr) => {
                int_test!($name, Specific u128 = [] u256 = [] i128 = [] i256 = [] $(attrs = $(#[$meta])*)? => $test_fn);
        };
        // Multiple optional tests with attrs.
         (
            $name:ident,
            Specific
            $(u128  = [$($pu128:expr),*])?
            $(u256  = [$($pu256:expr),*])?
            $(i128  = [$($pi128:expr),*])?
            $(i256  = [$($pi256:expr),*])?
            attrs = $(#[$meta:meta])*
            => $test_fn:expr
        ) => {
            paste::paste! {
                $(#[$meta])*
                #[test]
                fn [<$name _u128>]() {
                    $(
                        ($test_fn)(crate::Uint128::ZERO, $($pu128),*);
                    )?
                }

                $(#[$meta])*
                #[test]
                fn [<$name _u256>]() {
                    $(
                        ($test_fn)(crate::Uint256::ZERO, $($pu256),*);
                    )?
                }
                $(#[$meta])*

                #[test]
                fn [<$name _i128>]() {
                    $(
                        ($test_fn)(crate::Int128::ZERO, $($pi128),*);
                    )?
                }

                $(#[$meta])*
                #[test]
                fn [<$name _i256>]() {
                    $(
                        ($test_fn)(crate::Int256::ZERO, $($pi256),*);
                    )?
                }
            }
        };
         // Multiple optional tests without attrs.
         (
            $name:ident,
            Specific
            $(u128  = [$($pu128:expr),*])?
            $(u256  = [$($pu256:expr),*])?
            $(i128  = [$($pi128:expr),*])?
            $(i256  = [$($pi256:expr),*])?
            => $test_fn:expr
        ) => {
            paste::paste! {
                $(
                    #[test]
                    fn [<$name _u128>]() {
                        ($test_fn)(crate::Uint128::ZERO, $($pu128),*);
                    }
                )?

                $(
                    #[test]
                    fn [<$name _u256>]() {
                        ($test_fn)(crate::Uint256::ZERO, $($pu256),*);
                    }
                )?

                $(
                #[test]
                    fn [<$name _i128>]() {
                        ($test_fn)(crate::Int128::ZERO, $($pi128),*);
                    }
                )?

                $(
                    #[test]
                    fn [<$name _i256>]() {
                        ($test_fn)(crate::Int256::ZERO, $($pi256),*);
                    }
                )?
            }
        };
    }
