extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Span;
use std::marker;
use std::path::{Path, PathBuf};
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::{token, Token};
use wit_bindgen_core::{wit_parser::Interface, Files, WorldGenerator};
use wit_component::ComponentInterfaces;

pub fn generate<F, O>(
    input: TokenStream,
    mkgen: impl FnOnce(O) -> Box<dyn WorldGenerator>,
) -> TokenStream
where
    F: Parse + Configure<O>,
    O: Default,
{
    let input = syn::parse_macro_input!(input as Opts<F, O>);
    let mut gen = mkgen(input.opts);
    let mut files = Files::default();
    let name = match &input.name {
        Some(name) => name,
        None => {
            return Error::new(Span::call_site(), "must specify a `name` field")
                .to_compile_error()
                .into()
        }
    };
    gen.generate(name, &input.interfaces, &mut files);

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

    contents
}

pub trait Configure<O> {
    fn configure(self, opts: &mut O);
}

struct Opts<F, O> {
    opts: O,
    interfaces: ComponentInterfaces,
    name: Option<String>,
    files: Vec<String>,
    _marker: marker::PhantomData<F>,
}

mod kw {
    syn::custom_keyword!(import_str);
    syn::custom_keyword!(export_str);
    syn::custom_keyword!(default_str);
    syn::custom_keyword!(import);
    syn::custom_keyword!(export);
    syn::custom_keyword!(default);
    syn::custom_keyword!(name);
}

impl<F, O> Parse for Opts<F, O>
where
    F: Parse + Configure<O>,
    O: Default,
{
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let call_site = proc_macro2::Span::call_site();
        let mut ret = Opts {
            opts: O::default(),
            interfaces: ComponentInterfaces::default(),
            files: Vec::new(),
            name: None,
            _marker: marker::PhantomData,
        };

        if input.peek(token::Brace) {
            let content;
            syn::braced!(content in input);
            let fields = Punctuated::<ConfigField<F>, Token![,]>::parse_terminated(&content)?;
            for field in fields.into_pairs() {
                match field.into_value() {
                    ConfigField::Import(span, i) => ret.import(span, i)?,
                    ConfigField::ImportPath(name, path) => {
                        let span = path.span();
                        let interface = ret.parse(name, path)?;
                        ret.import(span, interface)?;
                    }
                    ConfigField::Export(span, i) => ret.export(span, i)?,
                    ConfigField::ExportPath(name, path) => {
                        let span = path.span();
                        let interface = ret.parse(name, path)?;
                        ret.export(span, interface)?;
                    }
                    ConfigField::Default(span, i) => ret.interface(span, i)?,
                    ConfigField::DefaultPath(name, path) => {
                        let span = path.span();
                        let interface = ret.parse(name, path)?;
                        ret.interface(span, interface)?;
                    }
                    ConfigField::Name(name) => {
                        if ret.name.is_some() {
                            return Err(Error::new(name.span(), "cannot specify `name` twice"));
                        }
                        ret.name = Some(name.value());
                    }
                    ConfigField::Other(other) => other.configure(&mut ret.opts),
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

impl<F, O> Opts<F, O> {
    fn parse(&mut self, name: Option<syn::LitStr>, path: syn::LitStr) -> Result<Interface> {
        let span = path.span();
        let path = path.value();
        let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let path = manifest_dir.join(path);
        self.files.push(path.to_str().unwrap().to_string());
        let mut file = Interface::parse_file(path).map_err(|e| Error::new(span, e))?;
        if let Some(name) = name {
            file.name = name.value();
        }
        Ok(file)
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

enum ConfigField<F> {
    Import(Span, Interface),
    ImportPath(Option<syn::LitStr>, syn::LitStr),
    Export(Span, Interface),
    ExportPath(Option<syn::LitStr>, syn::LitStr),
    Default(Span, Interface),
    DefaultPath(Option<syn::LitStr>, syn::LitStr),
    Name(syn::LitStr),
    Other(F),
}

impl<F: Parse> Parse for ConfigField<F> {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let l = input.lookahead1();
        if l.peek(kw::import_str) {
            let span = input.parse::<kw::import_str>()?.span;
            Ok(ConfigField::Import(span, parse_inline(input)?))
        } else if l.peek(kw::export_str) {
            let span = input.parse::<kw::export_str>()?.span;
            Ok(ConfigField::Export(span, parse_inline(input)?))
        } else if l.peek(kw::default_str) {
            let span = input.parse::<kw::default_str>()?.span;
            input.parse::<Token![:]>()?;
            Ok(ConfigField::Default(span, parse_inline(input)?))
        } else if l.peek(kw::import) {
            input.parse::<kw::import>()?;
            let name = parse_opt_name(input)?;
            input.parse::<Token![:]>()?;
            Ok(ConfigField::ImportPath(name, input.parse()?))
        } else if l.peek(kw::export) {
            input.parse::<kw::export>()?;
            let name = parse_opt_name(input)?;
            input.parse::<Token![:]>()?;
            Ok(ConfigField::ExportPath(name, input.parse()?))
        } else if l.peek(kw::default) {
            input.parse::<kw::default>()?;
            let name = parse_opt_name(input)?;
            input.parse::<Token![:]>()?;
            Ok(ConfigField::DefaultPath(name, input.parse()?))
        } else if l.peek(kw::name) {
            input.parse::<kw::name>()?;
            input.parse::<Token![:]>()?;
            Ok(ConfigField::Name(input.parse()?))
        } else {
            Ok(ConfigField::Other(input.parse()?))
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

fn parse_opt_name(input: ParseStream<'_>) -> Result<Option<syn::LitStr>> {
    if !input.peek(token::Bracket) {
        return Ok(None);
    }
    let name;
    syn::bracketed!(name in input);
    Ok(Some(name.parse::<syn::LitStr>()?))
}
