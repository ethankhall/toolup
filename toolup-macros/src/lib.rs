extern crate proc_macro;

mod helper;

use proc_macro::TokenStream;
use quote::quote;
use syn::*;

use crate::helper::*;

#[proc_macro_derive(ErrorCode, attributes(toolup))]
pub fn generate_error_code(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast = syn::parse(input).unwrap();

    // Build the trait implementation
    impl_generate_error_code(&ast)
}


fn impl_generate_error_code(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let meta = extract_meta(&ast.attrs);

    let variants = match ast.data {
        syn::Data::Enum(ref v) => &v.variants,
        _ => panic!("ErrorCode only works on Enums"),
    };

    let mut arms = Vec::new();

    let prefix = unique_attr(&meta, "toolup", "error_prefix").unwrap();

    for (idx, variant) in variants.iter().enumerate() {
        use syn::Fields::*;
        let ident = &variant.ident;

        let params = match variant.fields {
            Unit => quote!{},
            Unnamed(..) => quote!{ (..) },
            Named(..) => quote!{ {..} },
        };

        arms.push(quote!{ #name::#ident #params => format!("{}-{:03}", #prefix, #idx + 1)});
    }

    let tokens = quote! { 
        impl ErrorCode for #name {
            fn get_error_code(&self) -> String {
                match self {
                    #(#arms),*
                }
            }
        }
    };
    
    tokens.into()
}