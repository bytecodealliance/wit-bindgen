use proc_macro::TokenStream;
use proc_macro2::Span;
use std::path::{Path, PathBuf};
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::{token, Token};
use wit_bindgen_core::{wit_parser::Interface, Files};
use wit_component::ComponentInterfaces;

/// Generate code to support consuming the given interfaces, importaing them
/// from wasm modules.
#[proc_macro]
pub fn generate(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as Opts);
    let mut gen = input.opts.build();
    let mut files = Files::default();
    gen.generate("macro", &input.interfaces, &mut files);

    let (_, contents) = files.iter().next().unwrap();

    let contents = std::str::from_utf8(contents).unwrap();
    let mut contents = contents.parse::<TokenStream>().unwrap();

    // Include a dummy `include_str!` for any files we read so rustc knows that
    // we depend on the contents of those files.
    let cwd = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    for file in input.files.iter() {
        contents.extend(
            format!(
                "const _: &str = include_str!(r#\"{}\"#);\n",
                Path::new(&cwd).join(file).display()
            )
            .parse::<TokenStream>()
            .unwrap(),
        );
    }

    return contents;
}

#[derive(Default)]
struct Opts {
    opts: wit_bindgen_gen_host_wasmtime_rust::Opts,
    interfaces: ComponentInterfaces,
    files: Vec<String>,
}

mod kw {
    syn::custom_keyword!(import_str);
    syn::custom_keyword!(export_str);
    syn::custom_keyword!(default_str);
    syn::custom_keyword!(import);
    syn::custom_keyword!(export);
    syn::custom_keyword!(default);
    syn::custom_keyword!(tracing);
}

impl Parse for Opts {
    fn parse(input: ParseStream<'_>) -> Result<Opts> {
        let call_site = proc_macro2::Span::call_site();
        let mut ret = Opts::default();
        ret.opts.tracing = cfg!(feature = "tracing");

        if input.peek(token::Brace) {
            let content;
            syn::braced!(content in input);
            let fields = Punctuated::<ConfigField, Token![,]>::parse_terminated(&content)?;
            for field in fields.into_pairs() {
                match field.into_value() {
                    ConfigField::Import(span, i) => ret.import(span, i)?,
                    ConfigField::ImportPath(path) => {
                        let span = path.span();
                        let interface = ret.parse(path)?;
                        ret.import(span, interface)?;
                    }
                    ConfigField::Export(span, i) => ret.export(span, i)?,
                    ConfigField::ExportPath(path) => {
                        let span = path.span();
                        let interface = ret.parse(path)?;
                        ret.export(span, interface)?;
                    }
                    ConfigField::Default(span, i) => ret.interface(span, i)?,
                    ConfigField::DefaultPath(path) => {
                        let span = path.span();
                        let interface = ret.parse(path)?;
                        ret.interface(span, interface)?;
                    }
                    ConfigField::Tracing(v) => ret.opts.tracing = v,
                }
            }
        } else {
            while !input.is_empty() {
                let s = input.parse::<syn::LitStr>()?;
                ret.files.push(s.value());
            }
            return Err(Error::new(
                call_site,
                "string inputs won't be supported until `world` syntax is implemented",
            ));
        }
        Ok(ret)
    }
}

impl Opts {
    fn parse(&mut self, path: syn::LitStr) -> Result<Interface> {
        let span = path.span();
        let path = path.value();
        let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let path = manifest_dir.join(path);
        self.files.push(path.to_str().unwrap().to_string());
        Interface::parse_file(path).map_err(|e| Error::new(span, e))
    }

    fn import(&mut self, span: Span, i: Interface) -> Result<()> {
        match self.interfaces.imports.insert(i.name.clone(), i) {
            None => Ok(()),
            Some(_prev) => Err(Error::new(span, "duplicate import specified")),
        }
    }

    fn export(&mut self, span: Span, i: Interface) -> Result<()> {
        match self.interfaces.exports.insert(i.name.clone(), i) {
            None => Ok(()),
            Some(_prev) => Err(Error::new(span, "duplicate export specified")),
        }
    }

    fn interface(&mut self, span: Span, i: Interface) -> Result<()> {
        if self.interfaces.default.is_some() {
            return Err(Error::new(span, "duplicate default specified"));
        }
        self.interfaces.default = Some(i);
        Ok(())
    }
}

enum ConfigField {
    Import(Span, Interface),
    ImportPath(syn::LitStr),
    Export(Span, Interface),
    ExportPath(syn::LitStr),
    Default(Span, Interface),
    DefaultPath(syn::LitStr),
    Tracing(bool),
}

impl Parse for ConfigField {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let l = input.lookahead1();
        if l.peek(kw::import_str) {
            let span = input.parse::<kw::import>()?.span;
            Ok(ConfigField::Import(span, parse_inline(input)?))
        } else if l.peek(kw::export_str) {
            let span = input.parse::<kw::export>()?.span;
            Ok(ConfigField::Export(span, parse_inline(input)?))
        } else if l.peek(kw::default_str) {
            let span = input.parse::<kw::default>()?.span;
            input.parse::<Token![:]>()?;
            Ok(ConfigField::Default(span, parse_inline(input)?))
        } else if l.peek(kw::import) {
            input.parse::<kw::import>()?;
            input.parse::<Token![:]>()?;
            Ok(ConfigField::ImportPath(input.parse()?))
        } else if l.peek(kw::export) {
            input.parse::<kw::export>()?;
            input.parse::<Token![:]>()?;
            Ok(ConfigField::ExportPath(input.parse()?))
        } else if l.peek(kw::default) {
            input.parse::<kw::default>()?;
            input.parse::<Token![:]>()?;
            Ok(ConfigField::DefaultPath(input.parse()?))
        } else if l.peek(kw::tracing) {
            input.parse::<kw::tracing>()?;
            input.parse::<Token![:]>()?;
            Ok(ConfigField::Tracing(input.parse::<syn::LitBool>()?.value))
        } else {
            Err(l.error())
        }
    }
}

fn parse_inline(input: ParseStream<'_>) -> Result<Interface> {
    let name;
    syn::bracketed!(name in input);
    let name = name.parse::<syn::LitStr>()?;
    input.parse::<Token![:]>()?;
    let s = input.parse::<syn::LitStr>()?;
    Interface::parse(&name.value(), &s.value()).map_err(|e| Error::new(s.span(), e))
}
