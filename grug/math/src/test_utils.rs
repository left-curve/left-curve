use {
    crate::{Dec, Int},
    std::{fmt::Debug, str::FromStr},
};

/// `derive_type`
///
/// Allow compiler to derive the type of a variable,
/// which is necessary for the test functions.
pub(crate) fn dt<T>(_: T, _: T) {}

/// `derive_types`
///
///  Allow compiler to derive the types of multiple variables
#[macro_export(local_inner_macros)]
macro_rules! dts {
    ($u: expr, $($p:expr),*) => {
        $($crate::test_utils::dt($u, $p);)*
    };
}

/// `built_type`
///
///  Allow compiler to derive the type of a variable, and return right.
pub(crate) fn bt<T>(_: T, ret: T) -> T {
    ret
}

/// Combines `assert_eq` and `derive_type` to derive the type and assert
pub(crate) fn _smart_assert<T: Debug + PartialEq>(left: T, right: T) {
    assert_eq!(left, right);
}

// ------------------------------------ int ------------------------------------

/// Macro for unit tests for Int.
///
/// Is not possible to use [`test_case::test_case`] because the arguments types
/// can are different.
/// Also `Int<U>` is different for each test case.
///
/// The macro set as first parameter of the callback function `Int::ZERO`, so
/// the compiler can derive the type.
/// (See [`derive_type`], [`derive_types`] and [`smart_assert`].)
#[macro_export(local_inner_macros)]
macro_rules! int_test {
    // No Args
    (
        $name:ident
        $(attrs = $(#[$meta:meta])* $(,)?)?
        method = $test_fn:expr
    ) => {
        int_test!($name
            inputs = {
                u128 = []
                u256 = []
                i128 = []
                i256 = []
            }
            $(attrs = $(#[$meta])*)?
            method = $test_fn
        );
    };
    // Multiple optional tests with attrs.
    (
        $name:ident
        inputs = {
            $(u128 = [$($pu128:expr),*] $(,)?)?
            $(u256 = [$($pu256:expr),*] $(,)?)?
            $(i128 = [$($pi128:expr),*] $(,)?)?
            $(i256 = [$($pi256:expr),*] $(,)?)?
        } $(,)?
        attrs = $(#[$meta:meta])* $(,)?
        method = $test_fn:expr
    ) => {
        paste::paste! {
            $(#[$meta])*
            #[allow(clippy::just_underscores_and_digits)]
            #[test]
            fn [<$name _u128>]() {
                $(
                    ($test_fn)(<$crate::Uint128 as $crate::NumberConst>::ZERO, $($pu128),*);
                )?
            }

            $(#[$meta])*
            #[allow(clippy::just_underscores_and_digits)]
            #[test]
            fn [<$name _u256>]() {
                $(
                    ($test_fn)(<$crate::Uint256 as $crate::NumberConst>::ZERO, $($pu256),*);
                )?
            }

            $(#[$meta])*
            #[allow(clippy::just_underscores_and_digits)]
            #[test]
            fn [<$name _i128>]() {
                $(
                    ($test_fn)(<$crate::Int128 as $crate::NumberConst>::ZERO, $($pi128),*);
                )?
            }

            $(#[$meta])*
            #[allow(clippy::just_underscores_and_digits)]
            #[test]
            fn [<$name _i256>]() {
                $(
                    ($test_fn)(<$crate::Int256 as $crate::NumberConst>::ZERO, $($pi256),*);
                )?
            }
        }
    };
    // Multiple optional tests without attrs.
    (
        $name:ident
        inputs = {
            $(u128 = [$($pu128:expr),*] $(,)?)?
            $(u256 = [$($pu256:expr),*] $(,)?)?
            $(i128 = [$($pi128:expr),*] $(,)?)?
            $(i256 = [$($pi256:expr),*] $(,)?)?
        } $(,)?
        method = $test_fn:expr
    ) => {
        paste::paste! {
            $(
                #[test]
                #[allow(clippy::just_underscores_and_digits)]
                fn [<$name _u128>]() {
                    ($test_fn)(<$crate::Uint128 as $crate::NumberConst>::ZERO, $($pu128),*);
                }
            )?

            $(
                #[test]
                #[allow(clippy::just_underscores_and_digits)]
                fn [<$name _u256>]() {
                    ($test_fn)(<$crate::Uint256 as $crate::NumberConst>::ZERO, $($pu256),*);
                }
            )?

            $(
                #[test]
                #[allow(clippy::just_underscores_and_digits)]
                fn [<$name _i128>]() {
                    ($test_fn)(<$crate::Int128 as $crate::NumberConst>::ZERO, $($pi128),*);
                }
            )?

            $(
                #[test]
                #[allow(clippy::just_underscores_and_digits)]
                fn [<$name _i256>]() {
                    ($test_fn)(<$crate::Int256 as $crate::NumberConst>::ZERO, $($pi256),*);
                }
            )?
        }
    };
    // Multiple optional passing/failing tests.
    (
        $name:ident
        inputs = {
            $(u128 = {
                passing: [$($pu128:expr),* $(,)?] $(,)?
                $(failing: [$($fu128:expr),* $(,)?])? $(,)?
            } $(,)? )?
            $(u256 = {
                passing: [$($pu256:expr),* $(,)?] $(,)?
                $(failing: [$($fu256:expr),* $(,)?])? $(,)?
            } $(,)? )?
            $(i128 = {
                passing: [$($pi128:expr),* $(,)?] $(,)?
                $(failing: [$($fi128:expr),* $(,)?])? $(,)?
            } $(,)? )?
            $(i256 = {
                passing: [$($pi256:expr),* $(,)?] $(,)?
                $(failing: [$($fi256:expr),* $(,)?])? $(,)?
            } $(,)? )?
        } $(,)?
        $(attrs = $ (#[$meta:meta])*)? $(,)?
        method = $test_fn:expr
    ) => {
        int_test!(
            $name
            inputs = {
                $(u128 = [[$($pu128),*] $(, [$($fu128),*])?])?
                $(u256 = [[$($pu256),*] $(, [$($fu256),*])?])?
                $(i128 = [[$($pi128),*] $(, [$($fi128),*])?])?
                $(i256 = [[$($pi256),*] $(, [$($fi256),*])?])?
            }
            $(attrs = $(#[$meta])*)?
            method = $test_fn
        );
    };
}

/// Macro for unit tests for Int.
///
/// Is not possible to use [`test_case::test_case`] because the arguments types
/// can are different.
/// Also `Int<U>` is different for each test case.
///
/// The macro set as first parameter of the callback function `Int::ZERO`, so
/// the compiler can derive the type.
/// (See [`derive_type`], [`derive_types`] and [`smart_assert`].)
#[macro_export(local_inner_macros)]
macro_rules! dec_test {
    // No Args
    (
        $name:ident
        $(attrs = $(#[$meta:meta])* $(,)?)?
        method = $test_fn:expr
    ) => {
        dec_test!($name
            inputs = {
                udec128 = []
                udec256 = []
                dec128 = []
                dec256 = []
            }
            $(attrs = $(#[$meta])*)?
            method = $test_fn
        );
    };
    // Multiple optional tests with attrs.
    (
        $name:ident
        inputs = {
            $(udec128 = [$($pu128:expr),*] $(,)?)?
            $(udec256 = [$($pu256:expr),*] $(,)?)?
            $(dec128 =  [$($pi128:expr),*] $(,)?)?
            $(dec256 =  [$($pi256:expr),*] $(,)?)?
        } $(,)?
        attrs = $(#[$meta:meta])* $(,)?
        method = $test_fn:expr
    ) => {
        paste::paste! {
            $(#[$meta])*
            #[allow(clippy::just_underscores_and_digits)]
            #[test]
            fn [<$name _udec128>]() {
                $(
                    ($test_fn)(<$crate::Udec128 as $crate::NumberConst>::ZERO, $($pu128),*);
                )?
            }

            $(#[$meta])*
            #[allow(clippy::just_underscores_and_digits)]
            #[test]
            fn [<$name _udec256>]() {
                $(
                    ($test_fn)(<$crate::Udec256 as $crate::NumberConst>::ZERO, $($pu256),*);
                )?
            }

            $(#[$meta])*
            #[allow(clippy::just_underscores_and_digits)]
            #[test]
            fn [<$name _dec128>]() {
                $(
                    ($test_fn)(<$crate::Dec128 as $crate::NumberConst>::ZERO, $($pi128),*);
                )?
            }

            $(#[$meta])*
            #[allow(clippy::just_underscores_and_digits)]
            #[test]
            fn [<$name _dec256>]() {
                $(
                    ($test_fn)(<$crate::Dec256 as $crate::NumberConst>::ZERO, $($pi256),*);
                )?
            }
        }
    };
    // Multiple optional tests without attrs.
    (
        $name:ident
        inputs = {
            $(udec128 = [$($pu128:expr),*] $(,)?)?
            $(udec256 = [$($pu256:expr),*] $(,)?)?
            $(dec128 = [$($pi128:expr),*] $(,)?)?
            $(dec256 = [$($pi256:expr),*] $(,)?)?
        } $(,)?
        method = $test_fn:expr
    ) => {
        paste::paste! {
            $(
                #[test]
                #[allow(clippy::just_underscores_and_digits)]
                fn [<$name _udec128>]() {
                    ($test_fn)(<$crate::Udec128 as $crate::NumberConst>::ZERO, $($pu128),*);
                }
            )?

            $(
                #[test]
                #[allow(clippy::just_underscores_and_digits)]
                fn [<$name _udec256>]() {
                    ($test_fn)(<$crate::Udec256 as $crate::NumberConst>::ZERO, $($pu256),*);
                }
            )?

            $(
                #[test]
                #[allow(clippy::just_underscores_and_digits)]
                fn [<$name _dec128>]() {
                    ($test_fn)(<$crate::Dec128 as $crate::NumberConst>::ZERO, $($pi128),*);
                }
            )?

            $(
                #[test]
                #[allow(clippy::just_underscores_and_digits)]
                fn [<$name _dec256>]() {
                    ($test_fn)(<$crate::Dec256 as $crate::NumberConst>::ZERO, $($pi256),*);
                }
            )?
        }
    };
    // Multiple optional passing/failing tests.
    (
        $name:ident
        inputs = {
            $(udec128 = {
                passing: [$($pu128:expr),* $(,)?] $(,)?
                $(failing: [$($fu128:expr),* $(,)?])? $(,)?
            } $(,)? )?
            $(udec256 = {
                passing: [$($pu256:expr),* $(,)?] $(,)?
                $(failing: [$($fu256:expr),* $(,)?])? $(,)?
            } $(,)? )?
            $(dec128 = {
                passing: [$($pi128:expr),* $(,)?] $(,)?
                $(failing: [$($fi128:expr),* $(,)?])? $(,)?
            } $(,)? )?
            $(dec256 = {
                passing: [$($pi256:expr),* $(,)?] $(,)?
                $(failing: [$($fi256:expr),* $(,)?])? $(,)?
            } $(,)? )?
        } $(,)?
        $(attrs = $ (#[$meta:meta])*)? $(,)?
        method = $test_fn:expr
    ) => {
        dec_test!(
            $name
            inputs = {
                $(udec128 = [[$($pu128),*] $(, [$($fu128),*])?])?
                $(udec256 = [[$($pu256),*] $(, [$($fu256),*])?])?
                $(dec128 = [[$($pi128),*] $(, [$($fi128),*])?])?
                $(dec256 = [[$($pi256),*] $(, [$($fi256),*])?])?
            }
            $(attrs = $(#[$meta])*)?
            method = $test_fn
        );
    };
}

pub fn int<U>(val: &str) -> Int<U>
where
    Int<U>: FromStr,
    <Int<U> as FromStr>::Err: Debug,
{
    match Int::from_str(val) {
        Ok(int) => int,
        Err(err) => panic!("failed to parse integer `{val}` in test helper: {err:?}"),
    }
}

/// Shortcut for create a `Dec` from a string.
pub fn dec<U, const S: u32>(val: &str) -> Dec<U, S>
where
    Dec<U, S>: FromStr,
    <Dec<U, S> as FromStr>::Err: Debug,
{
    match Dec::from_str(val) {
        Ok(dec) => dec,
        Err(err) => panic!("failed to parse decimal `{val}` in test helper: {err:?}"),
    }
}
