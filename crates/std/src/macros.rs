/// Declare a bounded type with the given type and bounds.
///
/// ## Note
///
/// This has to be put here instead of in grug-types where `Bounded` is defined,
/// because it relies on the prelude `paste` re-exported by grug.
#[macro_export]
macro_rules! declare_bounded {
    (name = $name:ident,type = $type:ty,min = $min:expr,max = $max:expr $(,)?) => {
        ::grug::__private::paste::paste! {
            struct [<$name Bounds>];

            impl Bounds<$type> for [<$name Bounds>] {
                const MIN: Bound<$type> = $min;
                const MAX: Bound<$type> = $max;
            }

            type $name = Bounded<$type, [<$name Bounds>]>;
        }
    };
    (name = $name:ident,type = $type:ty,max = $max:expr $(,)?) => {
        declare_bounded! {
            name = $name,
            type = $type,
            min = Bound::Unbounded,
            max = $max,
        }
    };
    (name = $name:ident,type = $type:ty,min = $max:expr $(,)?) => {
        declare_bounded! {
            name = $name,
            type = $type,
            min = $min,
            max = Bound::Unbounded,
        }
    };
}
