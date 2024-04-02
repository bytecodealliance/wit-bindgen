use heck::*;
use proc_macro::TokenStream;
use std::env;

/// This macro invokes a local macro called `codegen_test!` with the name
/// of the test and the full path to the test to execute. The local
/// `codegen_test!` macro then does what's necessary to actually run the test.
#[proc_macro]
pub fn codegen_tests(_input: TokenStream) -> TokenStream {
    let tests_dir = env::current_dir().unwrap().join("tests/codegen");
    let tests = tests_dir
        .read_dir()
        .unwrap()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            let is_dir = entry.file_type().unwrap().is_dir();
            if is_dir || path.extension().and_then(|s| s.to_str()) == Some("wit") {
                let test_path = if is_dir {
                    path.join("wit")
                } else {
                    path.clone()
                };
                let name = path.file_stem().unwrap().to_str().unwrap();
                let ident = quote::format_ident!("{}", name.to_snake_case());
                let path = test_path.to_str().unwrap();
                Some(quote::quote! {
                    codegen_test!(#ident #name #path);
                })
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    (quote::quote!(#(#tests)*)).into()
}
