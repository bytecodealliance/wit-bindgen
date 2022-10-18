use proc_macro::TokenStream;
use syn::parse::{Parse, ParseStream, Result};
use syn::Token;
use wit_bindgen_gen_host_wasmtime_rust::Opts;

#[proc_macro]
pub fn generate(input: TokenStream) -> TokenStream {
    wit_bindgen_rust_macro_shared::generate::<Opt, Opts>(input, |opts| opts.build())
}

mod kw {
    syn::custom_keyword!(tracing);
}

enum Opt {
    Tracing(bool),
}

impl Parse for Opt {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let l = input.lookahead1();

        if l.peek(kw::tracing) {
            input.parse::<kw::tracing>()?;
            input.parse::<Token![:]>()?;
            Ok(Opt::Tracing(input.parse::<syn::LitBool>()?.value))
        } else {
            Err(l.error())
        }
    }
}

impl wit_bindgen_rust_macro_shared::Configure<Opts> for Opt {
    fn configure(self, opts: &mut Opts) {
        match self {
            Opt::Tracing(val) => opts.tracing = val,
        }
    }
}
