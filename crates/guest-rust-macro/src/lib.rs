use std::path::{Path, PathBuf};

use proc_macro::TokenStream;
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::{token, Token};
use wit_bindgen_core::{wit_parser::Interface, Direction, Files, Generator};

#[proc_macro]
pub fn import(input: TokenStream) -> TokenStream {
    run(input, Direction::Import)
}

#[proc_macro]
pub fn export(input: TokenStream) -> TokenStream {
    run(input, Direction::Export)
}

fn run(input: TokenStream, dir: Direction) -> TokenStream {
    let input = syn::parse_macro_input!(input as Opts);
    let mut gen = input.opts.build();
    let mut files = Files::default();
    let (imports, exports) = match dir {
        Direction::Import => (input.interfaces, vec![]),
        Direction::Export => (vec![], input.interfaces),
    };
    gen.generate_all(&imports, &exports, &mut files);
    let (_, contents) = files.iter().next().unwrap();
    let mut contents = std::str::from_utf8(contents).unwrap().to_string();

    // Include a dummy `include_str!` for any files we read so rustc knows that
    // we depend on the contents of those files.
    let cwd = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    for file in input.files.iter() {
        contents.push_str(&format!(
            "const _: &str = include_str!(r#\"{}\"#);\n",
            Path::new(&cwd).join(file).display()
        ));
    }

    contents.parse().unwrap()
}

struct Opts {
    opts: wit_bindgen_gen_guest_rust::Opts,
    interfaces: Vec<Interface>,
    files: Vec<String>,
}

mod kw {
    syn::custom_keyword!(src);
    syn::custom_keyword!(paths);
    syn::custom_keyword!(unchecked);
    syn::custom_keyword!(multi_module);
}

impl Parse for Opts {
    fn parse(input: ParseStream<'_>) -> Result<Opts> {
        let mut opts = wit_bindgen_gen_guest_rust::Opts::default();
        let call_site = proc_macro2::Span::call_site();
        let mut files = Vec::new();
        let interfaces = if input.peek(token::Brace) {
            let content;
            syn::braced!(content in input);
            let mut interfaces = Vec::new();
            let fields = Punctuated::<ConfigField, Token![,]>::parse_terminated(&content)?;
            for field in fields.into_pairs() {
                match field.into_value() {
                    ConfigField::Unchecked => opts.unchecked = true,
                    ConfigField::MultiModule => opts.multi_module = true,
                    ConfigField::Interfaces(v) => interfaces = v,
                }
            }
            if interfaces.is_empty() {
                return Err(Error::new(
                    call_site,
                    "must either specify `src` or `paths` keys",
                ));
            }
            interfaces
        } else {
            while !input.is_empty() {
                let s = input.parse::<syn::LitStr>()?;
                files.push(s.value());
            }
            let mut interfaces = Vec::new();
            let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
            for path in files.iter() {
                let path = manifest_dir.join(path);
                let iface = Interface::parse_file(path).map_err(|e| Error::new(call_site, e))?;
                interfaces.push(iface);
            }
            interfaces
        };
        Ok(Opts {
            files,
            opts,
            interfaces,
        })
    }
}

enum ConfigField {
    Interfaces(Vec<Interface>),
    Unchecked,
    MultiModule,
}

impl Parse for ConfigField {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let l = input.lookahead1();
        if l.peek(kw::src) {
            input.parse::<kw::src>()?;
            let name;
            syn::bracketed!(name in input);
            let name = name.parse::<syn::LitStr>()?;
            input.parse::<Token![:]>()?;
            let s = input.parse::<syn::LitStr>()?;
            let interface =
                Interface::parse(&name.value(), &s.value()).map_err(|e| Error::new(s.span(), e))?;
            Ok(ConfigField::Interfaces(vec![interface]))
        } else if l.peek(kw::paths) {
            input.parse::<kw::paths>()?;
            input.parse::<Token![:]>()?;
            let paths;
            let bracket = syn::bracketed!(paths in input);
            let paths = Punctuated::<syn::LitStr, Token![,]>::parse_terminated(&paths)?;
            let values = paths.iter().map(|s| s.value()).collect::<Vec<_>>();
            let mut interfaces = Vec::new();
            let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
            for value in &values {
                let value = manifest_dir.join(value);
                let interface =
                    Interface::parse_file(value).map_err(|e| Error::new(bracket.span, e))?;
                interfaces.push(interface);
            }
            Ok(ConfigField::Interfaces(interfaces))
        } else if l.peek(kw::unchecked) {
            input.parse::<kw::unchecked>()?;
            Ok(ConfigField::Unchecked)
        } else if l.peek(kw::multi_module) {
            input.parse::<kw::multi_module>()?;
            Ok(ConfigField::MultiModule)
        } else {
            Err(l.error())
        }
    }
}
