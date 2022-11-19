use proc_macro::TokenStream;
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::{Ident, Token};
use wit_bindgen_gen_host_wasmtime_rust::Opts;

#[proc_macro]
pub fn generate(input: TokenStream) -> TokenStream {
    wit_bindgen_rust_macro_shared::generate::<Opt, Opts>(input, |opts| opts.build())
}

mod kw {
    syn::custom_keyword!(tracing);
    syn::custom_keyword!(trappable_error_type);
}

enum Opt {
    Tracing(bool),
    Async(bool),
    TrappableErrorType(Vec<(String, String)>),
}

impl Parse for Opt {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let l = input.lookahead1();

        if l.peek(kw::tracing) {
            input.parse::<kw::tracing>()?;
            input.parse::<Token![:]>()?;
            Ok(Opt::Tracing(input.parse::<syn::LitBool>()?.value))
        } else if l.peek(Token![async]) {
            input.parse::<Token![async]>()?;
            input.parse::<Token![:]>()?;
            Ok(Opt::Async(input.parse::<syn::LitBool>()?.value))
        } else if l.peek(kw::trappable_error_type) {
            input.parse::<kw::trappable_error_type>()?;
            input.parse::<Token![:]>()?;
            let contents;
            syn::braced!(contents in input);
            let list = Punctuated::<ArrowBetween, Token![,]>::parse_terminated(&contents)?;
            Ok(Opt::TrappableErrorType(
                list.iter().map(|i| i.0.clone()).collect(),
            ))
        } else {
            Err(l.error())
        }
    }
}

struct ArrowBetween((String, String));
impl Parse for ArrowBetween {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let a = input.parse::<Ident>()?;
        let _ = input.parse::<Token![=>]>()?;
        let b = input.parse::<Ident>()?;
        Ok(ArrowBetween((a.to_string(), b.to_string())))
    }
}

impl wit_bindgen_rust_macro_shared::Configure<Opts> for Opt {
    fn configure(self, opts: &mut Opts) {
        match self {
            Opt::Tracing(val) => opts.tracing = val,
            Opt::Async(val) => opts.async_ = val,
            Opt::TrappableErrorType(val) => opts.trappable_error_type = val,
        }
    }
}
