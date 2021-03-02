use proc_macro::TokenStream;
use syn::parse::{Error, Parse, ParseStream, Result};
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

    let mut header = "
        use witx_bindgen_wasmtime::{wasmtime, anyhow};
    "
    .parse::<TokenStream>()
    .unwrap();
    let contents = contents.parse::<TokenStream>().unwrap();
    header.extend(contents);
    return header;
}

struct Opts {
    opts: witx_bindgen_gen_wasmtime::Opts,
    doc: witx::Document,
}

impl Parse for Opts {
    fn parse(input: ParseStream<'_>) -> Result<Opts> {
        let mut paths = Vec::new();
        while !input.is_empty() {
            let s = input.parse::<syn::LitStr>()?;
            paths.push(s.value());
        }
        let doc = witx::load(&paths)
            .map_err(|e| Error::new(proc_macro2::Span::call_site(), e.report()))?;
        Ok(Opts {
            opts: Default::default(),
            doc,
        })
    }
}
