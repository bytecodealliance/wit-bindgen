use heck::*;
use ignore::gitignore::GitignoreBuilder;
use proc_macro::{TokenStream, TokenTree};
use std::env;

/// This macro is invoked with a list of string literals as arguments which are
/// gitignore-style filters of tests to run in the `test/codegen` directory.
///
/// For example `codegen_tests!("foo.wit")` will run one tests and
/// `codegen_tests!("*.wit")` will run all tests.
///
/// This macro then invokes a local macro called `codegen_test!` with the name
/// of the test and the full path to the test to execute. The local
/// `codegen_test!` macro then does what's necessary to actually run the test.
#[proc_macro]
pub fn codegen_tests(input: TokenStream) -> TokenStream {
    let mut builder = GitignoreBuilder::new("tests");
    for token in input {
        let lit = match token {
            TokenTree::Literal(l) => l.to_string(),
            _ => panic!("invalid input"),
        };
        assert!(lit.starts_with("\""));
        assert!(lit.ends_with("\""));
        builder.add_line(None, &lit[1..lit.len() - 1]).unwrap();
    }
    let ignore = builder.build().unwrap();
    let cwd = env::current_dir().unwrap();
    let tests = ignore::Walk::new(cwd.join("tests/codegen"))
        .filter_map(|d| {
            let d = d.unwrap();
            let path = d.path();
            match ignore.matched(path, d.file_type().map(|d| d.is_dir()).unwrap_or(false)) {
                ignore::Match::None => None,
                ignore::Match::Ignore(_) => Some(d.into_path()),
                ignore::Match::Whitelist(_) => None,
            }
        })
        .map(|test| {
            let name = test.file_stem().unwrap().to_str().unwrap();
            let path = test.to_str().unwrap();
            let ident = quote::format_ident!("{}", name.to_snake_case());
            quote::quote! {
                codegen_test!(#ident #name #path);
            }
        });
    (quote::quote!(#(#tests)*)).into()
}
