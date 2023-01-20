use proc_macro2::{Span, TokenStream};
use std::path::{Path, PathBuf};
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::{token, Token};
use wit_bindgen_core::wit_parser::{PackageId, Resolve, UnresolvedPackage, WorldId};
use wit_bindgen_gen_guest_rust::Opts;

#[proc_macro]
pub fn generate(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    syn::parse_macro_input!(input as Config)
        .expand()
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

struct Config {
    opts: Opts,
    resolve: Resolve,
    world: WorldId,
    files: Vec<PathBuf>,
}

enum Source {
    Path(String),
    Inline(String),
}

impl Parse for Config {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let call_site = Span::call_site();
        let mut opts = Opts::default();
        let mut world = None;
        let mut source = None;

        let document = if input.peek(token::Brace) {
            let content;
            syn::braced!(content in input);
            let fields = Punctuated::<Opt, Token![,]>::parse_terminated(&content)?;
            let mut document = None;
            for field in fields.into_pairs() {
                match field.into_value() {
                    Opt::Path(s) => {
                        if source.is_some() {
                            return Err(Error::new(s.span(), "cannot specify second source"));
                        }
                        source = Some(Source::Path(s.value()));
                    }
                    Opt::World(s) => {
                        if document.is_some() {
                            return Err(Error::new(s.span(), "cannot specify second document"));
                        }
                        document = Some(parse_doc(&s.value(), &mut world));
                    }
                    Opt::Inline(s) => {
                        if source.is_some() {
                            return Err(Error::new(s.span(), "cannot specify second source"));
                        }
                        source = Some(Source::Inline(s.value()));
                    }
                    Opt::Unchecked => opts.unchecked = true,
                    Opt::NoStd => opts.no_std = true,
                    Opt::RawStrings => opts.raw_strings = true,
                    Opt::MacroExport => opts.macro_export = true,
                    Opt::MacroCallPrefix(prefix) => opts.macro_call_prefix = Some(prefix.value()),
                    Opt::ExportMacroName(name) => opts.export_macro_name = Some(name.value()),
                    Opt::Skip(list) => opts.skip.extend(list.iter().map(|i| i.value())),
                }
            }
            match (document, &source) {
                (Some(doc), _) => doc,
                (None, Some(Source::Inline(_))) => "macro-input".to_string(),
                _ => {
                    return Err(Error::new(
                        call_site,
                        "must specify a `world` to generate bindings for",
                    ))
                }
            }
        } else {
            let document = input.parse::<syn::LitStr>()?;
            if input.parse::<Option<syn::token::In>>()?.is_some() {
                source = Some(Source::Path(input.parse::<syn::LitStr>()?.value()));
            }
            parse_doc(&document.value(), &mut world)
        };
        let (resolve, pkg, files) =
            parse_source(&source).map_err(|err| Error::new(call_site, format!("{err:?}")))?;
        let doc = resolve.packages[pkg]
            .documents
            .get(&document)
            .copied()
            .ok_or_else(|| {
                Error::new(call_site, format!("no document named `{document}` found"))
            })?;

        let world = match &world {
            Some(name) => resolve.documents[doc]
                .worlds
                .get(name)
                .copied()
                .ok_or_else(|| Error::new(call_site, format!("no world named `{name}` found")))?,
            None => resolve.documents[doc].default_world.ok_or_else(|| {
                Error::new(call_site, format!("no default world found in `{document}`"))
            })?,
        };
        Ok(Config {
            opts,
            resolve,
            world,
            files,
        })
    }
}

fn parse_source(source: &Option<Source>) -> anyhow::Result<(Resolve, PackageId, Vec<PathBuf>)> {
    let mut resolve = Resolve::default();
    let mut files = Vec::new();
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let mut parse = |path: &Path| -> anyhow::Result<_> {
        if path.is_dir() {
            let (pkg, sources) = resolve.push_dir(&path)?;
            files = sources;
            Ok(pkg)
        } else {
            let pkg = UnresolvedPackage::parse_file(path)?;
            files.extend(pkg.source_files().map(|s| s.to_owned()));
            resolve.push(pkg, &Default::default())
        }
    };
    let pkg = match source {
        Some(Source::Inline(s)) => resolve.push(
            UnresolvedPackage::parse("macro-input".as_ref(), &s)?,
            &Default::default(),
        )?,
        Some(Source::Path(s)) => parse(&root.join(&s))?,
        None => parse(&root.join("wit"))?,
    };

    Ok((resolve, pkg, files))
}

fn parse_doc(s: &str, world: &mut Option<String>) -> String {
    match s.find('.') {
        Some(pos) => {
            *world = Some(s[pos + 1..].to_string());
            s[..pos].to_string()
        }
        None => s.to_string(),
    }
}

impl Config {
    fn expand(self) -> Result<TokenStream> {
        let mut files = Default::default();
        self.opts
            .build()
            .generate(&self.resolve, self.world, &mut files);
        let (_, src) = files.iter().next().unwrap();
        let src = std::str::from_utf8(src).unwrap();
        let mut contents = src.parse::<TokenStream>().unwrap();

        // Include a dummy `include_str!` for any files we read so rustc knows that
        // we depend on the contents of those files.
        for file in self.files.iter() {
            contents.extend(
                format!("const _: &str = include_str!(r#\"{}\"#);\n", file.display())
                    .parse::<TokenStream>()
                    .unwrap(),
            );
        }

        Ok(contents)
    }
}

mod kw {
    syn::custom_keyword!(unchecked);
    syn::custom_keyword!(no_std);
    syn::custom_keyword!(raw_strings);
    syn::custom_keyword!(macro_export);
    syn::custom_keyword!(macro_call_prefix);
    syn::custom_keyword!(export_macro_name);
    syn::custom_keyword!(skip);
    syn::custom_keyword!(world);
    syn::custom_keyword!(path);
    syn::custom_keyword!(inline);
}

enum Opt {
    World(syn::LitStr),
    Path(syn::LitStr),
    Inline(syn::LitStr),
    Unchecked,
    NoStd,
    RawStrings,
    MacroExport,
    MacroCallPrefix(syn::LitStr),
    ExportMacroName(syn::LitStr),
    Skip(Vec<syn::LitStr>),
}

impl Parse for Opt {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let l = input.lookahead1();
        if l.peek(kw::path) {
            input.parse::<kw::path>()?;
            input.parse::<Token![:]>()?;
            Ok(Opt::Path(input.parse()?))
        } else if l.peek(kw::inline) {
            input.parse::<kw::inline>()?;
            input.parse::<Token![:]>()?;
            Ok(Opt::Inline(input.parse()?))
        } else if l.peek(kw::world) {
            input.parse::<kw::world>()?;
            input.parse::<Token![:]>()?;
            Ok(Opt::World(input.parse()?))
        } else if l.peek(kw::unchecked) {
            input.parse::<kw::unchecked>()?;
            Ok(Opt::Unchecked)
        } else if l.peek(kw::no_std) {
            input.parse::<kw::no_std>()?;
            Ok(Opt::NoStd)
        } else if l.peek(kw::raw_strings) {
            input.parse::<kw::raw_strings>()?;
            Ok(Opt::RawStrings)
        } else if l.peek(kw::macro_export) {
            input.parse::<kw::macro_export>()?;
            Ok(Opt::MacroExport)
        } else if l.peek(kw::macro_call_prefix) {
            input.parse::<kw::macro_call_prefix>()?;
            input.parse::<Token![:]>()?;
            Ok(Opt::MacroCallPrefix(input.parse()?))
        } else if l.peek(kw::export_macro_name) {
            input.parse::<kw::export_macro_name>()?;
            input.parse::<Token![:]>()?;
            Ok(Opt::ExportMacroName(input.parse()?))
        } else if l.peek(kw::skip) {
            input.parse::<kw::skip>()?;
            input.parse::<Token![:]>()?;
            let contents;
            syn::bracketed!(contents in input);
            let list = Punctuated::<_, Token![,]>::parse_terminated(&contents)?;
            Ok(Opt::Skip(list.iter().cloned().collect()))
        } else {
            Err(l.error())
        }
    }
}
