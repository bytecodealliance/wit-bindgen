use proc_macro::TokenStream;
use syn::parse::{Error, Parse, ParseStream, Result};
use witx_bindgen_gen_core::{witx, Files, Generator};

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
    for module in input.modules {
        gen.generate(&module, import, &mut files);
    }
    let (_, contents) = files.iter().next().unwrap();

    let mut header = "
        use witx_bindgen_wasmtime::{wasmtime, anyhow, bitflags};
    "
    .parse::<TokenStream>()
    .unwrap();
    let contents = contents.parse::<TokenStream>().unwrap();
    header.extend(contents);
    return header;
}

struct Opts {
    opts: witx_bindgen_gen_wasmtime::Opts,
    modules: Vec<witx::Module>,
}

impl Parse for Opts {
    fn parse(input: ParseStream<'_>) -> Result<Opts> {
        let mut paths = Vec::new();
        while !input.is_empty() {
            let s = input.parse::<syn::LitStr>()?;
            paths.push(s.value());
        }
        let mut modules = Vec::new();
        for path in &paths {
            let module = witx::load(path)
                .map_err(|e| Error::new(proc_macro2::Span::call_site(), e.report()))?;
            modules.push(module);
        }
        Ok(Opts {
            opts: Default::default(),
            modules,
        })
    }
}
