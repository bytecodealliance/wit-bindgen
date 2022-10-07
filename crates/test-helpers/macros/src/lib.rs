use heck::*;
use ignore::gitignore::GitignoreBuilder;
use proc_macro::{TokenStream, TokenTree};
use std::env;

include!(concat!(env!("OUT_DIR"), "/wasms.rs"));

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
            let test = test.to_str().unwrap();
            let test_name = quote::format_ident!("{}", name.to_snake_case());
            quote::quote! {
                codegen_test!(#test_name #test);
            }
        });
    (quote::quote!(#(#tests)*)).into()
}

/// Invoked as `runtime_tests!("js")` to run a top-level `execute` function with
/// all host tests that use the "js" extension.
#[proc_macro]
pub fn runtime_tests(input: TokenStream) -> TokenStream {
    generate_runtime_tests(input, false)
}

/// Same as `runtime_tests!` but iterates over component wasms instead of core
/// wasms.
#[proc_macro]
pub fn runtime_component_tests(input: TokenStream) -> TokenStream {
    generate_runtime_tests(input, true)
}

fn generate_runtime_tests(input: TokenStream, use_components: bool) -> TokenStream {
    let host_extension = input.to_string();
    let host_extension = host_extension.trim_matches('"');
    let host_file = format!("host.{}", host_extension);
    let mut tests = Vec::new();
    let cwd = std::env::current_dir().unwrap();
    for entry in std::fs::read_dir(cwd.join("tests/runtime")).unwrap() {
        let entry = entry.unwrap().path();
        if !entry.join(&host_file).exists() {
            continue;
        }
        let name_str = entry.file_name().unwrap().to_str().unwrap();
        for (lang, name, wasm, component) in WASMS {
            if *name != name_str {
                continue;
            }
            let name = quote::format_ident!("{}_{}", name_str, lang);
            let host_file = entry.join(&host_file).to_str().unwrap().to_string();
            let import_wit = entry.join("imports.wit").to_str().unwrap().to_string();
            let export_wit = entry.join("exports.wit").to_str().unwrap().to_string();
            if use_components {
                tests.push(quote::quote! {
                    #[test]
                    fn #name() {
                        crate::execute(
                            #name_str,
                            #lang,
                            #component.as_ref(),
                            #host_file.as_ref(),
                        )
                    }
                });
            } else {
                let name_str = format!("{}_{}", name_str, lang);
                tests.push(quote::quote! {
                    #[test]
                    fn #name() {
                        crate::execute(
                            #name_str,
                            #wasm.as_ref(),
                            #host_file.as_ref(),
                            #import_wit.as_ref(),
                            #export_wit.as_ref(),
                        )
                    }
                });
            }
        }
    }

    (quote::quote!(#(#tests)*)).into()
}

#[proc_macro]
pub fn runtime_tests_wasmtime(_input: TokenStream) -> TokenStream {
    let mut tests = Vec::new();
    let cwd = std::env::current_dir().unwrap();
    for entry in std::fs::read_dir(cwd.join("tests/runtime")).unwrap() {
        let entry = entry.unwrap().path();
        if !entry.join("host.rs").exists() {
            continue;
        }
        let name_str = entry.file_name().unwrap().to_str().unwrap();
        for (lang, name, _wasm, component) in WASMS {
            if *name != name_str {
                continue;
            }
            let name = quote::format_ident!("{}_{}", name_str, lang);
            let host_file = entry.join("host.rs").to_str().unwrap().to_string();
            tests.push(quote::quote! {
                mod #name {
                    include!(#host_file);

                    #[test_log::test]
                    fn test() -> anyhow::Result<()> {
                        run(#component)
                    }
                }
            });
        }
    }

    (quote::quote!(#(#tests)*)).into()
}
