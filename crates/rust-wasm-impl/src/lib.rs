use proc_macro::TokenStream;
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::{token, Token};
use witx_bindgen_gen_core::{witx, Generator};

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
    let files = gen.generate(&input.doc, import);
    let (_, contents) = files.iter().next().unwrap();
    contents.parse().unwrap()
}

struct Opts {
    opts: witx_bindgen_gen_rust_wasm::Opts,
    doc: witx::Document,
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
        let doc = if input.peek(token::Brace) {
            let content;
            syn::braced!(content in input);
            let mut doc = None;
            let fields = Punctuated::<ConfigField, Token![,]>::parse_terminated(&content)?;
            for field in fields.into_pairs() {
                match field.into_value() {
                    ConfigField::Unchecked => opts.unchecked = true,
                    ConfigField::MultiModule => opts.multi_module = true,
                    ConfigField::Document(d) => doc = Some(d),
                }
            }
            doc.ok_or_else(|| Error::new(call_site, "must either specify `src` or `paths` keys"))?
        } else {
            let mut paths = Vec::new();
            while !input.is_empty() {
                let s = input.parse::<syn::LitStr>()?;
                paths.push(s.value());
            }
            witx::load(&paths).map_err(|e| Error::new(call_site, e.report()))?
        };
        Ok(Opts { opts, doc })
    }
}

enum ConfigField {
    Document(witx::Document),
    Unchecked,
    MultiModule,
}

impl Parse for ConfigField {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let l = input.lookahead1();
        if l.peek(kw::src) {
            input.parse::<kw::src>()?;
            input.parse::<Token![:]>()?;
            let s = input.parse::<syn::LitStr>()?;
            let doc = witx::parse(&s.value()).map_err(|e| Error::new(s.span(), e.report()))?;
            Ok(ConfigField::Document(doc))
        } else if l.peek(kw::paths) {
            input.parse::<kw::paths>()?;
            input.parse::<Token![:]>()?;
            let paths;
            let bracket = syn::bracketed!(paths in input);
            let paths = Punctuated::<syn::LitStr, Token![,]>::parse_terminated(&paths)?;
            let values = paths.iter().map(|s| s.value()).collect::<Vec<_>>();
            let doc = witx::load(&values).map_err(|e| Error::new(bracket.span, e.report()))?;
            Ok(ConfigField::Document(doc))
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
