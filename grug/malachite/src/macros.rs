#[macro_export]
macro_rules! ctx {
    ($ty:ident) => {
        <crate::context::Context as malachitebft_core_types::Context>::$ty
    };
    ($ty:ident :: $val:ident) => {
        <<crate::context::Context as malachitebft_core_types::Context>::$ty as malachitebft_core_types::$ty>::$val
    };
    ($ty:ident as $as_ty:ident :: $val:ident) => {
        <<crate::context::Context as malachitebft_core_types::Context>::$ty as malachitebft_core_types::$as_ty>::$val
    };
}
