use proc_macro::TokenStream;
use syn::{
    parse::{Parse, ParseStream, Result},
    LitStr, Token,
};
use wit_bindgen_gen_guest_rust::Opts;

#[proc_macro]
pub fn generate(input: TokenStream) -> TokenStream {
    wit_bindgen_rust_macro_shared::generate::<Opt, Opts>(input, |opts| opts.build())
}

mod kw {
    syn::custom_keyword!(unchecked);
    syn::custom_keyword!(no_std);
    syn::custom_keyword!(raw_strings);
    syn::custom_keyword!(macro_call_prefix);
}

enum Opt {
    Unchecked,
    NoStd,
    RawStrings,
    MacroCallPrefix(LitStr),
}

impl Parse for Opt {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let l = input.lookahead1();
        if l.peek(kw::unchecked) {
            input.parse::<kw::unchecked>()?;
            Ok(Opt::Unchecked)
        } else if l.peek(kw::no_std) {
            input.parse::<kw::no_std>()?;
            Ok(Opt::NoStd)
        } else if l.peek(kw::raw_strings) {
            input.parse::<kw::raw_strings>()?;
            Ok(Opt::RawStrings)
        } else if l.peek(kw::macro_call_prefix) {
            input.parse::<kw::macro_call_prefix>()?;
            input.parse::<Token![:]>()?;
            Ok(Opt::MacroCallPrefix(input.parse()?))
        } else {
            Err(l.error())
        }
    }
}

impl wit_bindgen_rust_macro_shared::Configure<Opts> for Opt {
    fn configure(self, opts: &mut Opts) {
        match self {
            Opt::Unchecked => opts.unchecked = true,
            Opt::NoStd => opts.no_std = true,
            Opt::RawStrings => opts.raw_strings = true,
            Opt::MacroCallPrefix(prefix) => opts.macro_call_prefix = Some(prefix.value()),
        }
    }
}
