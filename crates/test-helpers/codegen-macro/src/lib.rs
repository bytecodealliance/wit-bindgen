use heck::*;
use proc_macro::TokenStream;
use std::env;

/// This macro invokes a local macro called `codegen_test!` with the name
/// of the test and the full path to the test to execute. The local
/// `codegen_test!` macro then does what's necessary to actually run the test.
#[proc_macro]
pub fn codegen_tests(_input: TokenStream) -> TokenStream {
    let mut tests = Vec::new();
    for entry in env::current_dir()
        .unwrap()
        .join("tests/codegen")
        .read_dir()
        .unwrap()
    {
        let entry = entry.unwrap();
        let test = entry.path();

        if entry.file_type().unwrap().is_dir()
            || test.extension().and_then(|s| s.to_str()) == Some("wit")
        {
            let name = test.file_stem().unwrap().to_str().unwrap();
            let path = test.to_str().unwrap();
            let ident = quote::format_ident!("{}", name.to_snake_case());
            tests.push(quote::quote! {
                codegen_test!(#ident #name #path);
            });
        }
    }
    (quote::quote!(#(#tests)*)).into()
}
