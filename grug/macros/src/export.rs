use {
    proc_macro::TokenStream,
    std::str::FromStr,
    syn::{ItemFn, parse_macro_input},
};

pub fn process(mut input: TokenStream) -> TokenStream {
    let cloned = input.clone();
    let function = parse_macro_input!(cloned as ItemFn);
    let name = function.sig.ident.to_string();
    let args = function.sig.inputs.len();

    // E.g. "ptr0: usize, ptr1: usize, ptr2: usize, "
    let typed_ptrs = (0..args).fold(String::new(), |acc, i| format!("{acc}ptr{i}: usize, "));
    // E.g. "ptr0, ptr1, ptr2, "
    let ptrs = (0..args).fold(String::new(), |acc, i| format!("{acc}ptr{i}, "));

    // New module to avoid conflict of function names
    let new_code = format!(
        r##"
            #[cfg(target_arch = "wasm32")]
            mod __wasm_export_{name} {{
                #[no_mangle]
                extern "C" fn {name}({typed_ptrs}) -> usize {{
                    grug::do_{name}(&super::{name}, {ptrs})
                }}
            }}
        "##
    );

    let entry = TokenStream::from_str(&new_code).unwrap();
    input.extend(entry);
    input
}
