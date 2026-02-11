use {
    proc_macro::TokenStream,
    quote::{format_ident, quote},
    syn::{ItemFn, parse_macro_input},
};

pub fn process(mut input: TokenStream) -> TokenStream {
    let cloned = input.clone();
    let function = parse_macro_input!(cloned as ItemFn);
    let name = function.sig.ident;
    let args = function.sig.inputs.len();
    let module_name = format_ident!("__wasm_export_{}", name);
    let do_fn_name = format_ident!("do_{}", name);
    let ptrs = (0..args)
        .map(|i| format_ident!("ptr{i}"))
        .collect::<Vec<_>>();

    let entry = quote! {
        #[cfg(target_arch = "wasm32")]
        mod #module_name {
            #[unsafe(no_mangle)]
            extern "C" fn #name(#(#ptrs: usize),*) -> usize {
                grug::#do_fn_name(&super::#name, #(#ptrs),*)
            }
        }
    };

    input.extend(TokenStream::from(entry));
    input
}
