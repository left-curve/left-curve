use {
    proc_macro::TokenStream,
    quote::quote,
    syn::{Data, DeriveInput, Fields, parse_macro_input},
};

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

    let variant_idents: Vec<_> = variants.iter().map(|v| &v.ident).collect();

    let raw_keys_arms = variant_idents.iter().enumerate().map(|(i, ident)| {
        let idx = i as u8;
        quote! {
            #name::#ident => ::std::vec![::grug::RawKey::Fixed8([#idx])],
        }
    });

    let from_slice_arms = variant_idents.iter().enumerate().map(|(i, ident)| {
        let idx = i as u8;
        quote! {
            [#idx] => ::std::result::Result::Ok(#name::#ident),
        }
    });

    let variant_count = variants.len();
    let error_msg = format!(
        "invalid {name} key! must be 0..{}",
        variant_count.saturating_sub(1)
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
