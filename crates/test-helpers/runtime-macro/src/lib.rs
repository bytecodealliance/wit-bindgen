use proc_macro::{TokenStream, TokenTree};
use std::env;

include!(concat!(env!("OUT_DIR"), "/wasms.rs"));

/// Invoked as `runtime_component_tests!("js")` to run a top-level `execute`
/// function with all host tests that use the "js" extension.
#[proc_macro]
pub fn runtime_component_tests(input: TokenStream) -> TokenStream {
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
        for (lang, name, _wasm, component) in WASMS {
            if *name != name_str {
                continue;
            }
            let name = quote::format_ident!("{}_{}", name_str, lang);
            let host_file = entry.join(&host_file).to_str().unwrap().to_string();
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
        }
    }

    (quote::quote!(#(#tests)*)).into()
}
