use proc_macro::TokenStream;
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::{token, Token};
use witx_bindgen_gen_core::{witx2, Files, Generator};

#[proc_macro]
pub fn import(input: TokenStream) -> TokenStream {
    run(input, true)
}

#[proc_macro]
pub fn export(input: TokenStream) -> TokenStream {
    run(input, false)
}

fn run(input: TokenStream, import: bool) -> TokenStream {
    let input = syn::parse_macro_input!(input as Opts);
    let mut gen = input.opts.build();
    let mut files = Files::default();
    for iface in input.interfaces {
        gen.generate(&iface, import, &mut files);
    }
    let (_, contents) = files.iter().next().unwrap();
    contents.parse().unwrap()
}

struct Opts {
    opts: witx_bindgen_gen_rust_wasm::Opts,
    interfaces: Vec<witx2::Interface>,
}

mod kw {
    syn::custom_keyword!(src);
    syn::custom_keyword!(paths);
    syn::custom_keyword!(unchecked);
    syn::custom_keyword!(multi_module);
}

impl Parse for Opts {
    fn parse(input: ParseStream<'_>) -> Result<Opts> {
        let mut opts = witx_bindgen_gen_rust_wasm::Opts::default();
        let call_site = proc_macro2::Span::call_site();
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
            let mut paths = Vec::new();
            while !input.is_empty() {
                let s = input.parse::<syn::LitStr>()?;
                paths.push(s.value());
            }
            let mut interfaces = Vec::new();
            for path in &paths {
                let iface =
                    witx2::Interface::parse_file(&path).map_err(|e| Error::new(call_site, e))?;
                interfaces.push(iface);
            }
            interfaces
        };
        Ok(Opts { opts, interfaces })
    }
}

enum ConfigField {
    Interfaces(Vec<witx2::Interface>),
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
            let interface = witx2::Interface::parse(&name.value(), &s.value())
                .map_err(|e| Error::new(s.span(), e))?;
            Ok(ConfigField::Interfaces(vec![interface]))
        } else if l.peek(kw::paths) {
            input.parse::<kw::paths>()?;
            input.parse::<Token![:]>()?;
            let paths;
            let bracket = syn::bracketed!(paths in input);
            let paths = Punctuated::<syn::LitStr, Token![,]>::parse_terminated(&paths)?;
            let values = paths.iter().map(|s| s.value()).collect::<Vec<_>>();
            let mut interfaces = Vec::new();
            for value in &values {
                let interface =
                    witx2::Interface::parse_file(value).map_err(|e| Error::new(bracket.span, e))?;
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
