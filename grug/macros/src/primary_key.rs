use {
    proc_macro::TokenStream,
    quote::quote,
    std::collections::HashSet,
    syn::{Data, DeriveInput, Expr, Fields, Lit, Meta, parse_macro_input},
};

/// Try to extract a `#[primary_key(index = <u8>)]` attribute from a variant.
fn parse_index_attr(variant: &syn::Variant) -> Option<u8> {
    for attr in &variant.attrs {
        if !attr.path().is_ident("primary_key") {
            continue;
        }

        let Meta::NameValue(nv) = attr.parse_args().expect(
            "expected `#[primary_key(index = <u8>)]`",
        ) else {
            panic!("expected `#[primary_key(index = <u8>)]`");
        };

        if !nv.path.is_ident("index") {
            panic!("expected `#[primary_key(index = <u8>)]`");
        }

        let Expr::Lit(lit) = &nv.value else {
            panic!("expected a u8 literal in `#[primary_key(index = <u8>)]`");
        };

        let Lit::Int(int) = &lit.lit else {
            panic!("expected a u8 literal in `#[primary_key(index = <u8>)]`");
        };

        return Some(int.base10_parse::<u8>().expect("index must be a valid u8"));
    }

    None
}

pub fn process(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let variants = match &input.data {
        Data::Enum(data) => &data.variants,
        _ => panic!("PrimaryKey can only be derived for enums"),
    };

    // Ensure all variants are unit (no fields).
    for variant in variants {
        if !matches!(variant.fields, Fields::Unit) {
            panic!(
                "PrimaryKey derive only supports unit variants, but `{}` has fields",
                variant.ident
            );
        }
    }

    if variants.len() > 256 {
        panic!("PrimaryKey derive supports at most 256 variants");
    }

    // Collect explicit indices.
    let explicit_indices: Vec<Option<u8>> = variants.iter().map(|v| parse_index_attr(v)).collect();

    let has_any = explicit_indices.iter().any(|i| i.is_some());
    let has_all = explicit_indices.iter().all(|i| i.is_some());

    if has_any && !has_all {
        panic!(
            "PrimaryKey derive: either all variants must have `#[primary_key(index = ...)]` or none"
        );
    }

    // Resolve final indices: explicit or sequential.
    let indices: Vec<u8> = if has_all {
        explicit_indices.iter().map(|i| i.unwrap()).collect()
    } else {
        (0..variants.len() as u8).collect()
    };

    // Check for duplicates.
    let mut seen = HashSet::new();
    for (i, idx) in indices.iter().enumerate() {
        if !seen.insert(idx) {
            let variant_name = &variants.iter().nth(i).unwrap().ident;
            panic!(
                "PrimaryKey derive: duplicate index {} on variant `{}`",
                idx, variant_name
            );
        }
    }

    let variant_idents: Vec<_> = variants.iter().map(|v| &v.ident).collect();

    let raw_keys_arms = variant_idents.iter().zip(&indices).map(|(ident, idx)| {
        quote! {
            #name::#ident => ::std::vec![::grug::RawKey::Fixed8([#idx])],
        }
    });

    let from_slice_arms = variant_idents.iter().zip(&indices).map(|(ident, idx)| {
        quote! {
            [#idx] => ::std::result::Result::Ok(#name::#ident),
        }
    });

    let valid_indices: Vec<String> = indices.iter().map(|i| i.to_string()).collect();
    let error_msg = format!(
        "invalid {name} key! must be one of: {}",
        valid_indices.join("|")
    );

    quote! {
        impl ::grug::PrimaryKey for #name {
            type Output = Self;
            type Prefix = ();
            type Suffix = ();

            const KEY_ELEMS: u8 = 1;

            fn raw_keys(&self) -> ::std::vec::Vec<::grug::RawKey<'_>> {
                match self {
                    #(#raw_keys_arms)*
                }
            }

            fn from_slice(bytes: &[u8]) -> ::grug::StdResult<Self::Output> {
                match bytes {
                    #(#from_slice_arms)*
                    _ => ::std::result::Result::Err(::grug::StdError::deserialize::<
                        Self::Output,
                        _,
                        ::grug::Binary,
                    >(
                        "key",
                        #error_msg,
                        bytes.into(),
                    )),
                }
            }
        }
    }
    .into()
}
