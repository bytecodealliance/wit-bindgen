extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Span;
use std::marker;
use std::path::{Path, PathBuf};
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::{token, Token};
use wit_bindgen_core::{wit_parser::World, Files, WorldGenerator};

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
    gen.generate(&input.world, &mut files);

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

pub trait Configure<O> {
    fn configure(self, opts: &mut O);
}

struct Opts<F, O> {
    opts: O,
    world: World,
    files: Vec<String>,
    _marker: marker::PhantomData<F>,
}

mod kw {
    syn::custom_keyword!(path);
    syn::custom_keyword!(inline);
}

impl<F, O> Parse for Opts<F, O>
where
    F: Parse + Configure<O>,
    O: Default,
{
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let call_site = proc_macro2::Span::call_site();
        let mut world = None;
        let mut ret = Opts {
            opts: O::default(),
            world: World::default(),
            files: Vec::new(),
            _marker: marker::PhantomData,
        };

        if input.peek(token::Brace) {
            let content;
            syn::braced!(content in input);
            let fields = Punctuated::<ConfigField<F>, Token![,]>::parse_terminated(&content)?;
            for field in fields.into_pairs() {
                match field.into_value() {
                    ConfigField::Path(path) => {
                        if world.is_some() {
                            return Err(Error::new(path.span(), "cannot specify second world"));
                        }
                        world = Some(ret.parse(path)?);
                    }
                    ConfigField::Inline(span, w) => {
                        if world.is_some() {
                            return Err(Error::new(span, "cannot specify second world"));
                        }
                        world = Some(w);
                    }
                    ConfigField::Other(other) => other.configure(&mut ret.opts),
                }
            }
        } else {
            let s = input.parse::<syn::LitStr>()?;
            world = Some(ret.parse(s)?);
        }
        ret.world = world.ok_or_else(|| {
            Error::new(
                call_site,
                "must specify a `*.wit` file to generate bindings for",
            )
        })?;
        Ok(ret)
    }
}

impl<F, O> Opts<F, O> {
    fn parse(&mut self, path: syn::LitStr) -> Result<World> {
        let span = path.span();
        let path = path.value();
        let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let path = manifest_dir.join(path);
        self.files.push(path.to_str().unwrap().to_string());
        World::parse_file(path).map_err(|e| Error::new(span, e))
    }
}

enum ConfigField<F> {
    Path(syn::LitStr),
    Inline(Span, World),
    Other(F),
}

impl<F: Parse> Parse for ConfigField<F> {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let l = input.lookahead1();
        if l.peek(kw::path) {
            input.parse::<kw::path>()?;
            input.parse::<Token![:]>()?;
            Ok(ConfigField::Path(input.parse()?))
        } else if l.peek(kw::inline) {
            let span = input.parse::<kw::inline>()?.span;
            Ok(ConfigField::Inline(span, parse_inline(input)?))
        } else {
            Ok(ConfigField::Other(input.parse()?))
        }
    }
}

fn parse_inline(input: ParseStream<'_>) -> Result<World> {
    input.parse::<Token![:]>()?;
    let s = input.parse::<syn::LitStr>()?;
    World::parse("<macro-input>", &s.value()).map_err(|e| Error::new(s.span(), e))
}
